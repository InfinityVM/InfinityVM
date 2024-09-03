//! The orderbook for maintaining and matching limit orders of a single pair.

use std::collections::{BTreeMap, HashMap};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::{
    api::{Order, OrderFill},
    Error,
};

/// Orderbook type.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
pub struct OrderBook {
    /// All bid limit orders.
    pub bids: BTreeMap<u64, Vec<Order>>,
    /// All ask limit orders orders.
    pub asks: BTreeMap<u64, Vec<Order>>,
    /// Map of order ID to level. A level is orders that all have the same price
    pub oid_to_level: HashMap<u64, u64>,
}

fn fill_at_price_level(
    level: &mut Vec<Order>,
    taker_oid: u64,
    size: u64,
    is_buy: bool,
    taker_address: [u8; 20],
) -> (u64, Vec<OrderFill>) {
    let mut complete_fills = 0;
    let mut remaining_amount = size;
    let mut fills = vec![];

    for maker in level.iter_mut() {
        let mut fill = OrderFill { maker_oid: maker.oid, taker_oid, ..Default::default() };
        if is_buy {
            fill.buyer = taker_address;
            fill.seller = maker.address
        } else {
            fill.buyer = maker.address;
            fill.seller = taker_address;
        }

        if maker.size <= remaining_amount {
            complete_fills += 1;
            remaining_amount -= maker.size;
            fill.size = maker.size;
            fill.price = maker.limit_price;
            fills.push(fill);
            if remaining_amount == 0 {
                break;
            }
        } else {
            maker.size -= remaining_amount;
            fill.size = remaining_amount;
            remaining_amount = 0;
            fill.price = maker.limit_price;
            fills.push(fill);
            break;
        }
    }
    level.drain(..complete_fills);

    (remaining_amount, fills)
}

impl OrderBook {
    /// Get the max bid.
    pub fn bid_max(&self) -> u64 {
        if let Some(level) = self.bids.iter().next_back() {
            *level.0
        } else {
            0
        }
    }

    /// Get the mid bid.
    pub fn ask_min(&self) -> u64 {
        if let Some(level) = self.asks.iter().next() {
            *level.0
        } else {
            u64::MAX
        }
    }

    fn enqueue_order(&mut self, order: Order) {
        self.oid_to_level.insert(order.oid, order.limit_price);
        if order.is_buy {
            let level = self.bids.entry(order.limit_price).or_default();
            level.push(order);
        } else {
            let level = self.asks.entry(order.limit_price).or_default();
            level.push(order);
        }
    }

    /// Add a limit order.
    pub fn limit(&mut self, order: Order) -> (u64, Vec<OrderFill>) {
        let mut remaining_amount = order.size;
        let mut ask_min = self.ask_min();
        let mut bid_max = self.bid_max();
        let mut fills = vec![];
        if order.is_buy {
            if order.limit_price >= ask_min {
                while remaining_amount > 0 && order.limit_price >= ask_min {
                    let level = self.asks.get_mut(&ask_min).unwrap();
                    let (new_remaining_amount, new_fills) = fill_at_price_level(
                        level,
                        order.oid,
                        remaining_amount,
                        order.is_buy,
                        order.address,
                    );
                    remaining_amount = new_remaining_amount;
                    fills.extend(new_fills);
                    if level.is_empty() {
                        self.asks.remove(&ask_min);
                    }
                    if remaining_amount > 0 {
                        ask_min = self.ask_min();
                    }
                }
            }
        } else if order.limit_price <= bid_max {
            while remaining_amount > 0 && order.limit_price <= bid_max {
                let level = self.bids.get_mut(&bid_max).unwrap();
                let (new_remaining_amount, new_fills) = fill_at_price_level(
                    level,
                    order.oid,
                    remaining_amount,
                    order.is_buy,
                    order.address,
                );
                remaining_amount = new_remaining_amount;
                fills.extend(new_fills);
                if level.is_empty() {
                    self.bids.remove(&bid_max);
                }
                if remaining_amount > 0 {
                    bid_max = self.bid_max();
                }
            }
        }

        if remaining_amount > 0 {
            self.enqueue_order(order);
        }

        (remaining_amount, fills)
    }

    /// Cancel a limit order.
    pub fn cancel(&mut self, oid: u64) -> Result<Order, Error> {
        let level_price = self.oid_to_level.get(&oid).ok_or(Error::OrderDoesNotExist)?;
        let order = if self.bids.contains_key(level_price) {
            let level = self.bids.get_mut(level_price).ok_or(Error::OrderDoesNotExist)?;
            level
                .iter()
                .position(|o| o.oid == oid)
                .map(|i| level.remove(i))
                .ok_or(Error::OrderDoesNotExist)?
        } else if self.asks.contains_key(level_price) {
            let level = self.asks.get_mut(level_price).ok_or(Error::OrderDoesNotExist)?;
            level
                .iter()
                .position(|o| o.oid == oid)
                .map(|i| level.remove(i))
                .ok_or(Error::OrderDoesNotExist)?
        } else {
            return Err(Error::OrderDoesNotExist);
        };
        self.oid_to_level.remove(&oid);

        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_bid_ask {
        ($book:expr, $expected_bid:expr, $expected_ask:expr) => {
            assert_eq!($book.bid_max(), $expected_bid);
            assert_eq!($book.ask_min(), $expected_ask);
        };
    }

    #[test]
    fn test_bid_max() {
        let mut book = OrderBook::default();
        book.limit(Order::new(true, 10, 10, 1));
        book.limit(Order::new(true, 20, 10, 2));
        book.limit(Order::new(true, 30, 10, 3));
        assert_bid_ask!(book, 30, u64::MAX);
    }

    #[test]
    fn test_ask_min() {
        let mut book = OrderBook::default();
        book.limit(Order::new(false, 10, 10, 1));
        book.limit(Order::new(false, 20, 10, 2));
        book.limit(Order::new(false, 30, 10, 3));
        assert_bid_ask!(book, 0, 10);
    }

    #[test]
    fn test_crossing_bid_max() {
        let mut book = OrderBook::default();
        book.limit(Order::new(true, 10, 10, 1));
        book.limit(Order::new(true, 20, 10, 2));
        book.limit(Order::new(true, 30, 10, 3));
        book.limit(Order::new(false, 25, 10, 5));
        assert_bid_ask!(book, 20, u64::MAX);
    }

    #[test]
    fn test_crossing_ask_min() {
        let mut book = OrderBook::default();
        book.limit(Order::new(false, 10, 10, 1));
        book.limit(Order::new(false, 20, 10, 2));
        book.limit(Order::new(false, 30, 10, 3));
        book.limit(Order::new(true, 25, 10, 5));
        assert_bid_ask!(book, 0, 20);
    }

    #[test]
    fn test_resting_bid_ask() {
        let mut book = OrderBook::default();
        book.limit(Order::new(true, 10, 10, 1));
        book.limit(Order::new(true, 20, 10, 2));
        book.limit(Order::new(false, 30, 10, 3));
        book.limit(Order::new(false, 25, 10, 5));
        assert_bid_ask!(book, 20, 25);
    }

    #[test]
    fn test_fill_at_price_level() {
        let mut level = vec![Order::new(true, 10, 10, 1), Order::new(true, 10, 10, 2)];
        let (remaining_amount, fills) = fill_at_price_level(&mut level, 3, 10, true, [0; 20]);
        assert_eq!(remaining_amount, 0);
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].maker_oid, 1);
        assert_eq!(fills[0].taker_oid, 3);
        assert_eq!(fills[0].size, 10);
    }
}

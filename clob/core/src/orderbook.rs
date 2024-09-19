//! The orderbook for maintaining and matching limit orders of a single pair.

use std::collections::{BTreeMap, HashMap};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::api::{Order, OrderFill};

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
    oid_to_level: &mut HashMap<u64, u64>,
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
            oid_to_level.remove(&maker.oid);
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
    pub fn bid_max(&self) -> Option<u64> {
        self.bids.keys().next_back().copied()
    }

    /// Get the min bid.
    pub fn ask_min(&self) -> Option<u64> {
        self.asks.keys().next().copied()
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
    ///
    /// Note that the price of each fill is determined by the maker's price.
    pub fn limit(&mut self, order: Order) -> (u64, Vec<OrderFill>) {
        let mut remaining_amount = order.size;
        let mut fills = vec![];

        if order.is_buy {
            let valid_ask = |ask: Option<u64>, rem: u64| {
                ask.and_then(
                    |min| {
                        if order.limit_price >= min && rem > 0 {
                            Some(min)
                        } else {
                            None
                        }
                    },
                )
            };
            let mut ask_min = self.ask_min();
            while let Some(min) = valid_ask(ask_min, remaining_amount) {
                let level = self.asks.get_mut(&min).expect("above checks that min exists");
                let (new_remaining_amount, new_fills) = fill_at_price_level(
                    level,
                    order.oid,
                    remaining_amount,
                    order.is_buy,
                    order.address,
                    &mut self.oid_to_level,
                );
                remaining_amount = new_remaining_amount;
                fills.extend(new_fills);
                if level.is_empty() {
                    self.asks.remove(&min);
                }
                if remaining_amount > 0 {
                    ask_min = self.ask_min();
                }
            }
        } else {
            let valid_bid = |bid: Option<u64>, rem: u64| {
                bid.and_then(
                    |max| {
                        if order.limit_price <= max && rem > 0 {
                            Some(max)
                        } else {
                            None
                        }
                    },
                )
            };
            let mut bid_max = self.bid_max();
            while let Some(max) = valid_bid(bid_max, remaining_amount) {
                let level = self.bids.get_mut(&max).expect("above checks that max exists");
                let (new_remaining_amount, new_fills) = fill_at_price_level(
                    level,
                    order.oid,
                    remaining_amount,
                    order.is_buy,
                    order.address,
                    &mut self.oid_to_level,
                );
                remaining_amount = new_remaining_amount;

                fills.extend(new_fills);
                if level.is_empty() {
                    self.bids.remove(&max);
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

    /// Cancel a limit order. Returns `None` if `oid` did not correspond to an active order.
    pub fn cancel(&mut self, oid: u64) -> Option<Order> {
        self.oid_to_level.remove(&oid).and_then(|price| {
            if let Some(lvl) = self.bids.get_mut(&price) {
                lvl.iter().position(|o| o.oid == oid).map(|i| lvl.remove(i))
            } else if let Some(lvl) = self.asks.get_mut(&price) {
                lvl.iter().position(|o| o.oid == oid).map(|i| lvl.remove(i))
            } else {
                None
            }
        })
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
        assert_bid_ask!(book, Some(30), None);
    }

    #[test]
    fn test_ask_min() {
        let mut book = OrderBook::default();
        book.limit(Order::new(false, 10, 10, 1));
        book.limit(Order::new(false, 20, 10, 2));
        book.limit(Order::new(false, 30, 10, 3));
        assert_bid_ask!(book, None, Some(10));
    }

    #[test]
    fn test_crossing_bid_max() {
        let mut book = OrderBook::default();
        book.limit(Order::new(true, 10, 10, 1));
        book.limit(Order::new(true, 20, 10, 2));
        book.limit(Order::new(true, 30, 10, 3));
        let (remaining, fills) = book.limit(Order::new(false, 9, 19, 5));
        assert_eq!(remaining, 0);
        assert_eq!(
            fills,
            vec![
                OrderFill { maker_oid: 3, taker_oid: 5, size: 10, price: 30, ..Default::default() },
                OrderFill { maker_oid: 2, taker_oid: 5, size: 9, price: 20, ..Default::default() }
            ]
        );
        assert_bid_ask!(book, Some(20), None);
    }

    #[test]
    fn test_crossing_ask_min() {
        let mut book = OrderBook::default();
        book.limit(Order::new(false, 10, 10, 1));
        book.limit(Order::new(false, 20, 10, 2));
        book.limit(Order::new(false, 30, 10, 3));
        let (remaining, fills) = book.limit(Order::new(true, 25, 19, 5));
        assert_eq!(remaining, 0);
        assert_eq!(
            fills,
            vec![
                OrderFill { maker_oid: 1, taker_oid: 5, size: 10, price: 10, ..Default::default() },
                OrderFill { maker_oid: 2, taker_oid: 5, size: 9, price: 20, ..Default::default() }
            ]
        );
        assert_bid_ask!(book, None, Some(20));
    }

    #[test]
    fn test_resting_bid_ask() {
        let mut book = OrderBook::default();
        book.limit(Order::new(true, 10, 10, 1));
        book.limit(Order::new(true, 20, 10, 2));
        book.limit(Order::new(false, 30, 10, 3));
        book.limit(Order::new(false, 25, 10, 5));
        assert_bid_ask!(book, Some(20), Some(25));
    }

    #[test]
    fn exact_crossing_bid() {
        let mut book = OrderBook::default();
        book.limit(Order::new(false, 4, 100, 1));
        book.limit(Order::new(true, 1, 100, 2));
        let (remaining, fills) = book.limit(Order::new(true, 4, 100, 3));
        assert_eq!(remaining, 0);
        assert_eq!(
            fills,
            vec![OrderFill {
                maker_oid: 1,
                taker_oid: 3,
                size: 100,
                price: 4,
                ..Default::default()
            }]
        );
    }

    #[test]
    fn exact_crossing_ask() {
        let mut book = OrderBook::default();
        book.limit(Order::new(true, 4, 100, 1));
        book.limit(Order::new(true, 1, 100, 2));
        let (remaining, fills) = book.limit(Order::new(false, 4, 100, 3));
        assert_eq!(remaining, 0);
        assert_eq!(
            fills,
            vec![OrderFill {
                maker_oid: 1,
                taker_oid: 3,
                size: 100,
                price: 4,
                ..Default::default()
            }]
        );
    }

    #[test]
    fn test_fill_at_price_level() {
        let mut level = vec![Order::new(true, 10, 10, 1), Order::new(true, 10, 10, 2)];
        let mut oid_to_level = HashMap::new();
        oid_to_level.insert(1, 10);
        oid_to_level.insert(2, 10);
        let (remaining_amount, fills) =
            fill_at_price_level(&mut level, 3, 10, true, [0; 20], &mut oid_to_level);
        assert_eq!(remaining_amount, 0);
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].maker_oid, 1);
        assert_eq!(fills[0].taker_oid, 3);
        assert_eq!(fills[0].size, 10);
        assert!(!oid_to_level.contains_key(&1));
    }
}

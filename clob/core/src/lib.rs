//! Core logic and types of the `InfinityVM` CLOB.
//!
//! Note that everything in here needs to be able to target the ZKVM architecture.

use std::collections::HashMap;

use crate::api::AssetBalance;
use alloy::{
    primitives::{utils::keccak256, FixedBytes},
    sol_types::SolType,
};

use abi::StatefulProgramResult;
use api::{
    AddOrderRequest, AddOrderResponse, CancelOrderRequest, CancelOrderResponse, ClobResultDeltas,
    DepositDelta, DepositRequest, DepositResponse, Diff, OrderDelta, Request, Response,
    WithdrawDelta, WithdrawRequest, WithdrawResponse,
};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

pub mod api;
pub mod orderbook;

use crate::api::FillStatus;
use orderbook::OrderBook;

/// The state of the universe for the CLOB.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
pub struct ClobState {
    /// Next order ID.
    oid: u64,
    /// Base asset account balances.
    base: HashMap<[u8; 20], AssetBalance>,
    /// Quote asset account balances.
    quote: HashMap<[u8; 20], AssetBalance>,
    /// Active orderbook
    book: OrderBook,
    /// Fill status of orders, keyed by Order ID.
    order_status: HashMap<u64, FillStatus>,
}

impl ClobState {
    /// Get the oid.
    pub const fn oid(&self) -> u64 {
        self.oid
    }
    /// Get the base asset balances.
    pub const fn base_balances(&self) -> &HashMap<[u8; 20], AssetBalance> {
        &self.base
    }
    /// Get the quote asset balances.
    pub const fn quote_balances(&self) -> &HashMap<[u8; 20], AssetBalance> {
        &self.quote
    }
    /// Get the book
    pub const fn book(&self) -> &OrderBook {
        &self.book
    }
    /// Get the order status.
    pub const fn order_status(&self) -> &HashMap<u64, FillStatus> {
        &self.order_status
    }
}

/// Deposit user funds that can be used to place orders.
pub fn deposit(req: DepositRequest, mut state: ClobState) -> (DepositResponse, ClobState, Diff) {
    // We enforce ensure that both balance types always exist.
    state
        .base
        .entry(req.address)
        .and_modify(|b| b.free += req.base_free)
        .or_insert(AssetBalance { free: req.base_free, locked: 0 });

    state
        .quote
        .entry(req.address)
        .and_modify(|b| b.free += req.quote_free)
        .or_insert(AssetBalance { free: req.quote_free, locked: 0 });

    (
        DepositResponse { success: true },
        state,
        Diff::deposit(req.address, req.base_free, req.quote_free),
    )
}

/// Withdraw non-locked funds.
pub fn withdraw(req: WithdrawRequest, mut state: ClobState) -> (WithdrawResponse, ClobState, Diff) {
    let a = req.address;

    if let (Some(base), Some(quote)) = (state.base.get_mut(&a), state.quote.get_mut(&a)) {
        if base.free < req.base_free || quote.free < req.quote_free {
            (WithdrawResponse { success: false }, state, Diff::Noop)
        } else {
            base.free -= req.base_free;
            quote.free -= req.quote_free;

            if *quote == AssetBalance::default() && *base == AssetBalance::default() {
                state.quote.remove(&a);
                state.base.remove(&a);
            }

            let change = Diff::withdraw(req.address, req.base_free, req.quote_free);
            (WithdrawResponse { success: true }, state, change)
        }
    } else {
        (WithdrawResponse { success: false }, state, Diff::Noop)
    }
}

/// Cancel an order.
pub fn cancel_order(
    req: CancelOrderRequest,
    mut state: ClobState,
) -> (CancelOrderResponse, ClobState, Diff) {
    let o = match state.book.cancel(req.oid) {
        Some(o) => o,
        None => {
            return (CancelOrderResponse { success: false, fill_status: None }, state, Diff::Noop)
        }
    };

    let fill_status = match state.order_status.remove(&req.oid) {
        Some(fill_status) => fill_status,
        None => {
            return (CancelOrderResponse { success: false, fill_status: None }, state, Diff::Noop)
        }
    };

    let change = if o.is_buy {
        debug_assert!(state.quote.contains_key(&o.address));
        if let Some(quote) = state.quote.get_mut(&o.address) {
            let remaining_quote = o.quote_size() - fill_status.filled_quote();
            quote.free += remaining_quote;
            quote.locked -= remaining_quote;
            Diff::cancel(o.address, 0, o.quote_size())
        } else {
            debug_assert!(false);
            return (CancelOrderResponse { success: false, fill_status: None }, state, Diff::Noop);
        }
    } else if let Some(base) = state.base.get_mut(&o.address) {
        let remaining_base = o.size - fill_status.filled_size;
        base.free += remaining_base;
        base.locked -= remaining_base;
        Diff::cancel(o.address, o.size, 0)
    } else {
        debug_assert!(false);
        return (CancelOrderResponse { success: false, fill_status: None }, state, Diff::Noop);
    };

    (CancelOrderResponse { success: true, fill_status: Some(fill_status) }, state, change)
}

/// Add an order.
///
/// Note that the price of each fill is determined by the maker's price.
pub fn add_order(
    req: AddOrderRequest,
    mut state: ClobState,
) -> (AddOrderResponse, ClobState, Vec<Diff>) {
    let (base, quote) = match (state.base.get_mut(&req.address), state.quote.get_mut(&req.address))
    {
        (Some(base), Some(quote)) => (base, quote),
        _ => return (AddOrderResponse { success: false, status: None }, state, vec![Diff::Noop]),
    };

    let o = req.to_order(state.oid);
    let order_id = o.oid;
    state.oid += 1;

    let is_invalid_buy = o.is_buy && quote.free < o.quote_size();
    let is_invalid_sell = !o.is_buy && base.free < o.size;
    if is_invalid_buy || is_invalid_sell {
        return (AddOrderResponse { success: false, status: None }, state, vec![Diff::Noop]);
    };

    let create = if req.is_buy {
        state.quote.entry(req.address).and_modify(|b| {
            b.free -= o.quote_size();
            b.locked += o.quote_size();
        });
        Diff::create(req.address, 0, o.quote_size())
    } else {
        state.base.entry(req.address).and_modify(|b| {
            b.free -= o.size;
            b.locked += o.size;
        });
        Diff::create(req.address, o.size, 0)
    };

    let (remaining_amount, fills) = state.book.limit(o);

    let mut changes = Vec::<Diff>::with_capacity(fills.len() + 1);
    changes.push(create);
    for fill in fills.iter().cloned() {
        if req.is_buy {
            // Seller exchanges base for quote
            state.base.entry(fill.seller).and_modify(|b| b.locked -= fill.size);
            state.quote.entry(fill.seller).and_modify(|b| b.free += fill.quote_size());

            // Buyer exchanges quote for base
            state.base.entry(req.address).and_modify(|b| b.free += fill.size);
            state.quote.entry(req.address).and_modify(|b| b.locked -= fill.quote_size());

            changes.push(Diff::fill(req.address, fill.seller, fill.size, fill.quote_size()));
        } else {
            // Seller exchanges base for quote
            state.base.entry(req.address).and_modify(|b| b.locked -= fill.size);
            state.quote.entry(req.address).and_modify(|b| b.free += fill.quote_size());

            // Buyer exchanges quote for base
            state.base.entry(fill.buyer).and_modify(|b| b.free += fill.size);
            state.quote.entry(fill.buyer).and_modify(|b| b.locked -= fill.quote_size());

            changes.push(Diff::fill(fill.buyer, req.address, fill.size, fill.quote_size()));
        }

        if let Some(make_order_status) = state.order_status.get_mut(&fill.maker_oid) {
            make_order_status.filled_size += fill.size;
            if make_order_status.filled_size < make_order_status.size {
                make_order_status.fills.push(fill);
            } else {
                // Remove filled orders
                state.order_status.remove(&fill.maker_oid);
            };
        };
    }

    let fill_size = req.size - remaining_amount;
    let fill_status = FillStatus {
        oid: order_id,
        size: req.size,
        filled_size: fill_size,
        fills,
        address: req.address,
    };

    if remaining_amount != 0 {
        // Only insert the status if its pending
        state.order_status.insert(order_id, fill_status.clone());
    }

    let resp = AddOrderResponse { success: true, status: Some(fill_status) };

    (resp, state, changes)
}

/// A tick will execute a single request against the CLOB state.
pub fn tick(request: Request, state: ClobState) -> (Response, ClobState, Vec<Diff>) {
    match request {
        Request::AddOrder(req) => {
            let (resp, state, diffs) = add_order(req, state);
            (Response::AddOrder(resp), state, diffs)
        }
        Request::CancelOrder(req) => {
            let (resp, state, diff) = cancel_order(req, state);
            (Response::CancelOrder(resp), state, vec![diff])
        }
        Request::Deposit(req) => {
            let (resp, state, diff) = deposit(req, state);
            (Response::Deposit(resp), state, vec![diff])
        }
        Request::Withdraw(req) => {
            let (resp, state, diff) = withdraw(req, state);
            (Response::Withdraw(resp), state, vec![diff])
        }
    }
}

/// State transition function used in the ZKVM. It only outputs balance changes, which are abi
/// encoded for contract consumption.
pub fn zkvm_stf(requests: Vec<Request>, mut state: ClobState) -> StatefulProgramResult {
    // At most 2 deltas for a request (fills have delta for buyer and seller)
    let mut orders = HashMap::<[u8; 20], OrderDelta>::with_capacity(requests.len() * 2);
    let mut deposits = HashMap::<[u8; 20], DepositDelta>::with_capacity(requests.len());
    let mut withdraws = HashMap::<[u8; 20], WithdrawDelta>::with_capacity(requests.len());

    for req in requests {
        let (_, next_state, diffs) = tick(req, state);
        for diff in diffs {
            diff.apply(&mut withdraws, &mut deposits, &mut orders);
        }

        state = next_state
    }

    let mut order_deltas: Vec<_> = orders.into_values().collect();
    order_deltas.sort();
    let mut withdraw_deltas: Vec<_> = withdraws.into_values().collect();
    withdraw_deltas.sort();
    let mut deposit_deltas: Vec<_> = deposits.into_values().collect();
    deposit_deltas.sort();

    let clob_result_deltas = ClobResultDeltas { order_deltas, withdraw_deltas, deposit_deltas };

    StatefulProgramResult {
        next_state_hash: state.borsh_keccak256(),
        result: ClobResultDeltas::abi_encode(&clob_result_deltas).into(),
    }
}

/// Trait for the sha256 hash of a borsh serialized type.
pub trait BorshSha256 {
    /// The sha256 hash of a borsh serialized type
    fn borsh_sha256(&self) -> [u8; 32];
}

// Blanket impl. for any type that implements borsh serialize.
impl<T: BorshSerialize> BorshSha256 for T {
    fn borsh_sha256(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        borsh::to_writer(&mut hasher, &self).expect("T is serializable");
        let hash = hasher.finalize();
        hash.into()
    }
}

/// Trait for the keccak256 hash of a borsh serialized type
pub trait BorshKeccak256 {
    /// The keccak256 hash of a borsh serialized type
    fn borsh_keccak256(&self) -> FixedBytes<32>;
}

impl<T: BorshSerialize> BorshKeccak256 for T {
    fn borsh_keccak256(&self) -> FixedBytes<32> {
        let borsh = borsh::to_vec(&self).expect("T is serializable");
        keccak256(&borsh)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        api::{
            AddOrderRequest, AddOrderResponse, AssetBalance, CancelOrderRequest,
            CancelOrderResponse, DepositRequest, DepositResponse, Diff, FillStatus, OrderFill,
            Request, Response, WithdrawRequest, WithdrawResponse,
        },
        tick, ClobState,
    };

    #[test]
    fn deposit_order_withdraw_cancel() {
        let clob_state = ClobState::default();
        let bob = [69u8; 20];
        let alice = [42u8; 20];

        let alice_dep =
            Request::Deposit(DepositRequest { address: alice, base_free: 200, quote_free: 0 });
        let (resp, clob_state, diffs) = tick(alice_dep, clob_state);
        assert_eq!(Response::Deposit(DepositResponse { success: true }), resp);
        assert_eq!(*clob_state.base.get(&alice).unwrap(), AssetBalance { free: 200, locked: 0 });
        assert_eq!(diffs, vec![Diff::deposit(alice, 200, 0)]);

        let bob_dep =
            Request::Deposit(DepositRequest { address: bob, base_free: 0, quote_free: 800 });
        let (resp, clob_state, diffs) = tick(bob_dep, clob_state);
        assert_eq!(Response::Deposit(DepositResponse { success: true }), resp);
        assert_eq!(*clob_state.quote.get(&bob).unwrap(), AssetBalance { free: 800, locked: 0 });
        assert_eq!(diffs, vec![Diff::deposit(bob, 0, 800)]);

        let alice_limit = Request::AddOrder(AddOrderRequest {
            address: alice,
            is_buy: false,
            limit_price: 4,
            size: 100,
        });
        let (resp, clob_state, diffs) = tick(alice_limit, clob_state);
        assert_eq!(
            Response::AddOrder(AddOrderResponse {
                success: true,
                status: Some(FillStatus {
                    oid: 0,
                    size: 100,
                    address: alice,
                    filled_size: 0,
                    fills: vec![]
                })
            }),
            resp
        );
        assert_eq!(*clob_state.base.get(&alice).unwrap(), AssetBalance { free: 100, locked: 100 });
        assert_eq!(diffs, vec![Diff::create(alice, 100, 0)]);

        let bob_limit1 = Request::AddOrder(AddOrderRequest {
            address: bob,
            is_buy: true,
            limit_price: 1,
            size: 100,
        });
        let (resp, clob_state, diffs) = tick(bob_limit1, clob_state);
        assert_eq!(
            Response::AddOrder(AddOrderResponse {
                success: true,
                status: Some(FillStatus {
                    oid: 1,
                    size: 100,
                    address: bob,
                    filled_size: 0,
                    fills: vec![]
                })
            }),
            resp
        );
        assert_eq!(*clob_state.quote.get(&bob).unwrap(), AssetBalance { free: 700, locked: 100 });
        assert_eq!(diffs, vec![Diff::create(bob, 0, 100)]);

        let bob_limit2 = Request::AddOrder(AddOrderRequest {
            address: bob,
            is_buy: true,
            limit_price: 4,
            size: 100,
        });
        let (resp, clob_state, diffs) = tick(bob_limit2, clob_state);
        assert_eq!(
            Response::AddOrder(AddOrderResponse {
                success: true,
                status: Some(FillStatus {
                    oid: 2,
                    size: 100,
                    address: bob,
                    filled_size: 100,
                    fills: vec![OrderFill {
                        maker_oid: 0,
                        taker_oid: 2,
                        buyer: bob,
                        seller: alice,
                        price: 4,
                        size: 100
                    }]
                })
            }),
            resp
        );
        assert_eq!(*clob_state.base.get(&alice).unwrap(), AssetBalance { free: 100, locked: 0 });
        assert_eq!(*clob_state.quote.get(&alice).unwrap(), AssetBalance { free: 400, locked: 0 });
        assert_eq!(*clob_state.base.get(&bob).unwrap(), AssetBalance { free: 100, locked: 0 });
        assert_eq!(*clob_state.quote.get(&bob).unwrap(), AssetBalance { free: 300, locked: 100 });
        assert_eq!(diffs, vec![Diff::create(bob, 0, 400), Diff::fill(bob, alice, 100, 400)]);

        let alice_withdraw =
            Request::Withdraw(WithdrawRequest { address: alice, base_free: 100, quote_free: 400 });
        let (resp, clob_state, diffs) = tick(alice_withdraw, clob_state);
        assert_eq!(Response::Withdraw(WithdrawResponse { success: true }), resp);
        assert!(!clob_state.quote.contains_key(&alice));
        assert!(!clob_state.base.contains_key(&alice));
        assert_eq!(diffs, vec![Diff::withdraw(alice, 100, 400)]);

        let bob_cancel = Request::CancelOrder(CancelOrderRequest { oid: 1 });
        let (resp, clob_state, diffs) = tick(bob_cancel, clob_state);
        assert_eq!(
            Response::CancelOrder(CancelOrderResponse {
                success: true,
                fill_status: Some(FillStatus {
                    oid: 1,
                    size: 100,
                    address: bob,
                    filled_size: 0,
                    fills: vec![]
                })
            }),
            resp
        );
        assert_eq!(*clob_state.quote.get(&bob).unwrap(), AssetBalance { free: 400, locked: 0 });
        assert_eq!(diffs, vec![Diff::cancel(bob, 0, 100)]);

        let bob_withdraw =
            Request::Withdraw(WithdrawRequest { address: bob, base_free: 100, quote_free: 400 });
        let (resp, clob_state, diffs) = tick(bob_withdraw, clob_state);
        assert_eq!(Response::Withdraw(WithdrawResponse { success: true }), resp);
        assert!(clob_state.quote.is_empty());
        assert!(clob_state.base.is_empty());
        assert_eq!(diffs, vec![Diff::withdraw(bob, 100, 400)]);
    }
}

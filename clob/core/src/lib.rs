//! Core logic and types of the `InfinityVM` CLOB.
//!
//! Note that everything in here needs to be able to target the ZKVM architecture

use std::collections::HashMap;

use crate::api::AssetBalance;
use alloy_primitives::{FixedBytes, Keccak256};
use api::{
    AddOrderRequest, AddOrderResponse, CancelOrderRequest, CancelOrderResponse, ClobProgramOutput,
    DepositDelta, DepositRequest, DepositResponse, Dif, OrderDelta, Request, Response,
    WithdrawDelta, WithdrawRequest, WithdrawResponse,
};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

pub mod api;
pub mod orderbook;

use crate::api::FillStatus;
use orderbook::OrderBook;

/// Errors for this crate.
#[derive(Clone, Debug, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub enum Error {
    /// An order could not be found
    OrderDoesNotExist,
}

/// Input to the STF. Expected to be the exact input given to the ZKVM program.
pub type StfInput = (Request, ClobState);
/// Output from the STF. Expected to be the exact output from the ZKVM program.
pub type StfOutput = (Response, ClobState);

/// The state of the universe for the CLOB.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
pub struct ClobState {
    oid: u64,
    base_balances: HashMap<[u8; 20], AssetBalance>,
    quote_balances: HashMap<[u8; 20], AssetBalance>,
    book: OrderBook,
    // TODO: ensure we are wiping order status for filled orders
    order_status: HashMap<u64, FillStatus>,
}

impl ClobState {
    /// Get the oid.
    pub const fn oid(&self) -> u64 {
        self.oid
    }
    /// Get the base asset balances.
    pub const fn base_balances(&self) -> &HashMap<[u8; 20], AssetBalance> {
        &self.base_balances
    }
    /// Get the quote asset balances.
    pub const fn quote_balances(&self) -> &HashMap<[u8; 20], AssetBalance> {
        &self.quote_balances
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
pub fn deposit(req: DepositRequest, mut state: ClobState) -> (DepositResponse, ClobState, Dif) {
    state
        .base_balances
        .entry(req.address)
        .and_modify(|b| b.free += req.base_free)
        .or_insert(AssetBalance { free: req.base_free, locked: 0 });

    state
        .quote_balances
        .entry(req.address)
        .and_modify(|b| b.free += req.quote_free)
        .or_insert(AssetBalance { free: req.quote_free, locked: 0 });

    (
        DepositResponse { success: true },
        state,
        Dif::deposit(req.address, req.base_free, req.quote_free),
    )
}

/// Withdraw non-locked funds
pub fn withdraw(req: WithdrawRequest, mut state: ClobState) -> (WithdrawResponse, ClobState, Dif) {
    let addr = req.address;
    let base_balance = state.base_balances.get_mut(&addr).expect("TODO");
    let quote_balance = state.quote_balances.get_mut(&addr).expect("TODO");

    if base_balance.free < req.base_free || quote_balance.free < req.quote_free {
        (WithdrawResponse { success: false }, state, Dif::Noop)
    } else {
        base_balance.free -= req.base_free;
        quote_balance.free -= req.quote_free;

        if *quote_balance == AssetBalance::default() && *base_balance == AssetBalance::default() {
            state.quote_balances.remove(&addr);
            state.base_balances.remove(&addr);
        }

        let change = Dif::withdraw(req.address, req.base_free, req.quote_free);
        (WithdrawResponse { success: true }, state, change)
    }
}

/// Cancel an order.
pub fn cancel_order(
    req: CancelOrderRequest,
    mut state: ClobState,
) -> (CancelOrderResponse, ClobState, Dif) {
    let o = match state.book.cancel(req.oid) {
        Ok(o) => o,
        Err(_) => {
            return (CancelOrderResponse { success: false, fill_status: None }, state, Dif::Noop)
        }
    };

    let change = if o.is_buy {
        let quote_balances = state.quote_balances.get_mut(&o.address).expect("todo");
        let quote_size = o.quote_size();
        quote_balances.free += quote_size;
        quote_balances.locked -= quote_size;
        Dif::cancel(o.address, 0, quote_size)
    } else {
        let base_balance = state.base_balances.get_mut(&o.address).expect("todo");
        base_balance.free += o.size;
        base_balance.locked -= o.size;
        Dif::cancel(o.address, o.size, 0)
    };

    let fill_status = state.order_status.remove(&o.oid);
    (CancelOrderResponse { success: true, fill_status }, state, change)
}

/// Add an order.
pub fn add_order(
    req: AddOrderRequest,
    mut state: ClobState,
) -> (AddOrderResponse, ClobState, Vec<Dif>) {
    let base_balance = state
        .base_balances
        .get(&req.address)
        .expect("todo: depositing ensures base balance exists");
    let quote_balance = state
        .quote_balances
        .get(&req.address)
        .expect("todo: depositing ensures quote balance exists");

    let o = req.to_order(state.oid);
    let order_id = o.oid;
    state.oid += 1;

    let is_invalid_buy = o.is_buy && quote_balance.free < o.quote_size();
    let is_invalid_sell = !o.is_buy && base_balance.free < o.size;
    if is_invalid_buy || is_invalid_sell {
        return (AddOrderResponse { success: false, status: None }, state, vec![Dif::Noop]);
    };

    let create = if req.is_buy {
        state.quote_balances.entry(req.address).and_modify(|b| {
            b.free -= o.quote_size();
            b.locked += o.quote_size();
        });
        Dif::create(req.address, 0, o.quote_size())
    } else {
        state.base_balances.entry(req.address).and_modify(|b| {
            b.free -= o.size;
            b.locked += o.size;
        });
        Dif::create(req.address, o.size, 0)
    };

    let (remaining_amount, fills) = state.book.limit(o);

    let mut changes = Vec::<Dif>::with_capacity(fills.len() + 1);
    changes.push(create);
    for fill in fills.iter().cloned() {
        let maker_order_status = state
            .order_status
            .get_mut(&fill.maker_oid)
            .expect("fill status is created when order is added");
        maker_order_status.filled_size += fill.size;

        if req.is_buy {
            // Seller exchanges base for quote
            state.base_balances.entry(fill.seller).and_modify(|b| b.locked -= fill.size);
            state.quote_balances.entry(fill.seller).and_modify(|b| b.free += fill.quote_size());

            // Buyer exchanges quote for base
            state.base_balances.entry(req.address).and_modify(|b| b.free += fill.size);
            state.quote_balances.entry(req.address).and_modify(|b| b.locked -= fill.quote_size());

            changes.push(Dif::fill(req.address, fill.seller, fill.size, fill.quote_size()));
        } else {
            // Seller exchanges base for quote
            state.base_balances.entry(req.address).and_modify(|b| b.locked -= fill.size);
            state.quote_balances.entry(req.address).and_modify(|b| b.free += fill.quote_size());

            // Buyer exchanges quote for base
            state.base_balances.entry(fill.buyer).and_modify(|b| b.free += fill.size);
            state.quote_balances.entry(fill.buyer).and_modify(|b| b.locked -= fill.quote_size());

            changes.push(Dif::fill(fill.buyer, req.address, fill.size, fill.quote_size()));
        }
        maker_order_status.fills.push(fill);
    }

    let fill_size = req.size - remaining_amount;
    let fill_status = FillStatus {
        oid: order_id,
        size: req.size,
        filled_size: fill_size,
        fills,
        address: req.address,
    };
    state.order_status.insert(order_id, fill_status.clone());

    let resp = AddOrderResponse { success: true, status: Some(fill_status) };

    (resp, state, changes)
}

/// A tick is will execute a single request against the CLOB state.
pub fn tick(request: Request, state: ClobState) -> Result<(Response, ClobState, Vec<Dif>), Error> {
    match request {
        Request::AddOrder(req) => {
            let (resp, state, difs) = add_order(req, state);
            Ok((Response::AddOrder(resp), state, difs))
        }
        Request::CancelOrder(req) => {
            let (resp, state, dif) = cancel_order(req, state);
            Ok((Response::CancelOrder(resp), state, vec![dif]))
        }
        Request::Deposit(req) => {
            let (resp, state, dif) = deposit(req, state);
            Ok((Response::Deposit(resp), state, vec![dif]))
        }
        Request::Withdraw(req) => {
            let (resp, state, dif) = withdraw(req, state);
            Ok((Response::Withdraw(resp), state, vec![dif]))
        }
    }
}

/// State transition function used in the ZKVM. It only outputs balance changes, which are abi
/// encoded for contract consumption.
pub fn zkvm_stf(requests: Vec<Request>, mut state: ClobState) -> ClobProgramOutput {
    // At most 2 deltas for a request (fills have delta for buyer and seller)
    let mut orders = HashMap::<[u8; 20], OrderDelta>::with_capacity(requests.len() * 2);
    let mut deposits = HashMap::<[u8; 20], DepositDelta>::with_capacity(requests.len());
    let mut withdraws = HashMap::<[u8; 20], WithdrawDelta>::with_capacity(requests.len());

    for req in requests {
        let (_, next_state, difs) = tick(req, state).expect("todo");
        for dif in difs {
            dif.apply(&mut withdraws, &mut deposits, &mut orders);
        }

        state = next_state
    }

    let mut order_deltas: Vec<_> = orders.into_values().collect();
    order_deltas.sort();
    let mut withdraw_deltas: Vec<_> = withdraws.into_values().collect();
    withdraw_deltas.sort();
    let mut deposit_deltas: Vec<_> = deposits.into_values().collect();
    deposit_deltas.sort();

    ClobProgramOutput {
        order_deltas,
        withdraw_deltas,
        deposit_deltas,
        next_state_hash: state.borsh_keccak256(),
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

/// Trait for the keccack256 hash of a borsh serialized type
pub trait BorshKeccack256 {
    /// The keccack256 hash of a borsh serialized type
    fn borsh_keccak256(&self) -> FixedBytes<32>;
}

impl<T: BorshSerialize> BorshKeccack256 for T {
    fn borsh_keccak256(&self) -> FixedBytes<32> {
        let borsh = borsh::to_vec(&self).expect("T is serializable");
        let mut hasher = Keccak256::new();
        hasher.update(borsh);
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        api::{
            AddOrderRequest, AddOrderResponse, AssetBalance, CancelOrderRequest,
            CancelOrderResponse, DepositRequest, DepositResponse, Dif, FillStatus, OrderFill,
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
        let (resp, clob_state, difs) = tick(alice_dep, clob_state).unwrap();
        assert_eq!(Response::Deposit(DepositResponse { success: true }), resp);
        assert_eq!(
            *clob_state.base_balances.get(&alice).unwrap(),
            AssetBalance { free: 200, locked: 0 }
        );
        assert_eq!(difs, vec![Dif::deposit(alice, 200, 0)]);

        let bob_dep =
            Request::Deposit(DepositRequest { address: bob, base_free: 0, quote_free: 800 });
        let (resp, clob_state, difs) = tick(bob_dep, clob_state).unwrap();
        assert_eq!(Response::Deposit(DepositResponse { success: true }), resp);
        assert_eq!(
            *clob_state.quote_balances.get(&bob).unwrap(),
            AssetBalance { free: 800, locked: 0 }
        );
        assert_eq!(difs, vec![Dif::deposit(bob, 0, 800)]);

        let alice_limit = Request::AddOrder(AddOrderRequest {
            address: alice,
            is_buy: false,
            limit_price: 4,
            size: 100,
        });
        let (resp, clob_state, difs) = tick(alice_limit, clob_state).unwrap();
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
        assert_eq!(
            *clob_state.base_balances.get(&alice).unwrap(),
            AssetBalance { free: 100, locked: 100 }
        );
        assert_eq!(difs, vec![Dif::create(alice, 100, 0)]);

        let bob_limit1 = Request::AddOrder(AddOrderRequest {
            address: bob,
            is_buy: true,
            limit_price: 1,
            size: 100,
        });
        let (resp, clob_state, difs) = tick(bob_limit1, clob_state).unwrap();
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
        assert_eq!(
            *clob_state.quote_balances.get(&bob).unwrap(),
            AssetBalance { free: 700, locked: 100 }
        );
        assert_eq!(difs, vec![Dif::create(bob, 0, 100)]);

        let bob_limit2 = Request::AddOrder(AddOrderRequest {
            address: bob,
            is_buy: true,
            limit_price: 4,
            size: 100,
        });
        let (resp, clob_state, difs) = tick(bob_limit2, clob_state).unwrap();
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
        assert_eq!(
            *clob_state.base_balances.get(&alice).unwrap(),
            AssetBalance { free: 100, locked: 0 }
        );
        assert_eq!(
            *clob_state.quote_balances.get(&alice).unwrap(),
            AssetBalance { free: 400, locked: 0 }
        );
        assert_eq!(
            *clob_state.base_balances.get(&bob).unwrap(),
            AssetBalance { free: 100, locked: 0 }
        );
        assert_eq!(
            *clob_state.quote_balances.get(&bob).unwrap(),
            AssetBalance { free: 300, locked: 100 }
        );
        assert_eq!(difs, vec![Dif::create(bob, 0, 400), Dif::fill(bob, alice, 100, 400)]);

        let alice_withdraw =
            Request::Withdraw(WithdrawRequest { address: alice, base_free: 100, quote_free: 400 });
        let (resp, clob_state, difs) = tick(alice_withdraw, clob_state).unwrap();
        assert_eq!(Response::Withdraw(WithdrawResponse { success: true }), resp);
        assert!(!clob_state.quote_balances.contains_key(&alice));
        assert!(!clob_state.base_balances.contains_key(&alice));
        assert_eq!(difs, vec![Dif::withdraw(alice, 100, 400)]);

        let bob_cancel = Request::CancelOrder(CancelOrderRequest { oid: 1 });
        let (resp, clob_state, difs) = tick(bob_cancel, clob_state).unwrap();
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
        assert_eq!(
            *clob_state.quote_balances.get(&bob).unwrap(),
            AssetBalance { free: 400, locked: 0 }
        );
        assert_eq!(difs, vec![Dif::cancel(bob, 0, 100)]);

        let bob_withdraw =
            Request::Withdraw(WithdrawRequest { address: bob, base_free: 100, quote_free: 400 });
        let (resp, clob_state, difs) = tick(bob_withdraw, clob_state).unwrap();
        assert_eq!(Response::Withdraw(WithdrawResponse { success: true }), resp);
        assert!(clob_state.quote_balances.is_empty());
        assert!(clob_state.base_balances.is_empty());
        assert_eq!(difs, vec![Dif::withdraw(bob, 100, 400)]);
    }
}

//! Types in the public API

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

mod balance_change;

pub use balance_change::{
    ClobProgramInput, ClobProgramOutput, DepositDelta, Dif, OrderDelta, WithdrawDelta,
};

/// All possible requests that can go into the clob engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub enum Request {
    /// [`AddOrderRequest`]
    AddOrder(AddOrderRequest),
    /// [`CancelOrderRequest`]
    CancelOrder(CancelOrderRequest),
    /// [`DepositRequest`]
    Deposit(DepositRequest),
    /// [`WithdrawRequest`]
    Withdraw(WithdrawRequest),
}

/// All possible responses from the clob engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub enum Response {
    /// [`AddOrderResponse`]
    AddOrder(AddOrderResponse),
    /// [`AddOrderResponse`]
    CancelOrder(CancelOrderResponse),
    /// [`DepositResponse`]
    Deposit(DepositResponse),
    /// [`WithdrawResponse`]
    Withdraw(WithdrawResponse),
}

/// A response from the clob engine with the global index of the request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse {
    /// The response from processing the request with one engine tick
    pub response: Response,
    /// The global index of the request. The request is guranteed to be processed
    /// via ordering indicated by this index
    pub global_index: u64,
}

/// Add a limit order.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct AddOrderRequest {
    /// Account placing the order.
    pub address: [u8; 20],
    /// If this is a buy or sell order
    pub is_buy: bool,
    /// The price to execute the order at
    pub limit_price: u64,
    /// The size of the asset
    pub size: u64,
}

impl AddOrderRequest {
    /// Convert the order request to an [Order].
    pub const fn to_order(&self, oid: u64) -> Order {
        Order {
            is_buy: self.is_buy,
            limit_price: self.limit_price,
            size: self.size,
            oid,
            address: self.address,
        }
    }
}

/// Response to [`AddOrderRequest`]
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct AddOrderResponse {
    /// If the request was fully processed.
    pub success: bool,
    /// Any fills that happened when placing the order.
    pub status: Option<FillStatus>,
    // TODO: OID
}

/// Cancel a limit order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderRequest {
    /// Order ID.
    pub oid: u64,
}

/// Response to [`CancelOrderRequest`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderResponse {
    /// If the request was fully processed.
    pub success: bool,
    /// Any fills from the cancelled ordered that have already occurred.
    pub fill_status: Option<FillStatus>,
}

/// Deposit funds that can be use to place orders.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct DepositRequest {
    /// Account to credit funds to.
    pub address: [u8; 20],
    /// Amount of base asset to credit.
    pub base_free: u64,
    /// Amount of quote asset to credit.
    pub quote_free: u64,
}

/// Response to [`DepositRequest`]
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct DepositResponse {
    /// If the request was fully processed.
    pub success: bool,
}

/// Withdraw non locked funds.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawRequest {
    /// Account to debit funds from
    pub address: [u8; 20],
    /// Amount to withdraw from base asset free balance
    pub base_free: u64,
    /// Amount to withdraw from quote asset free balance
    pub quote_free: u64,
}

/// Response to [`WithdrawRequest`].
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawResponse {
    /// If the request was fully processed.
    pub success: bool,
}

/// All balances for a user.
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct AssetBalance {
    /// Funds that are in the CLOB state, but not committed to orders.
    ///
    /// These funds can either be `lock`ed by a limit order or exit clob state
    /// by being moved to withdraw.
    ///
    /// To increase `free` funds, either some order needs to be filled or the
    /// user deposits into the clob.
    pub free: u64,
    /// Funds that are locked in orders. Funds can be moved to `free` by cancelling an order(s).
    /// When an order is matched, locked funds are transferred to the counter party.
    pub locked: u64,
}

/// A limit order.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    /// If the order is buy or sell.
    pub is_buy: bool,
    /// The price to execute the order at.
    pub limit_price: u64,
    /// Size of the asset to exchange.
    pub size: u64,
    /// Order ID.
    pub oid: u64,
    /// User that placed the order.
    pub address: [u8; 20],
}

impl Order {
    /// Create a new order, only used as a convenience in tests.
    #[cfg(test)]
    pub const fn new(is_buy: bool, limit_price: u64, size: u64, oid: u64) -> Self {
        Self { is_buy, limit_price, size, oid, address: [0; 20] }
    }

    /// Size of the quote asset required to fill this order.
    pub fn quote_size(&self) -> u64 {
        self.size * self.limit_price
    }
}

/// That current status of how filled an order is.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct FillStatus {
    /// Order ID
    pub oid: u64,
    /// Size of the order
    pub size: u64,
    /// Account that owns the order
    pub address: [u8; 20],
    /// The amount of the order that has been filled.
    pub filled_size: u64,
    /// Each fill that has been executed.
    pub fills: Vec<OrderFill>,
}

/// A match of two orders.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct OrderFill {
    /// Maker order ID.
    pub maker_oid: u64,
    /// Taker order ID.
    pub taker_oid: u64,
    /// Size of the base asset exchanged.
    pub size: u64,
    /// Price of the base asset in terms of quote asset.
    pub price: u64,
    /// Address of buyer
    pub buyer: [u8; 20],
    /// Address of the seller
    pub seller: [u8; 20],
}

impl OrderFill {
    /// Create a new [Self].
    pub const fn new(
        maker_oid: u64,
        taker_oid: u64,
        size: u64,
        price: u64,
        buyer: [u8; 20],
        seller: [u8; 20],
    ) -> Self {
        Self { maker_oid, taker_oid, size, price, buyer, seller }
    }

    /// Size of the quote asset exchanged
    pub fn quote_size(&self) -> u64 {
        self.size * self.price
    }
}

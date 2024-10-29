//! Tables for the database.

use crate::db::models::{RequestModel, ResponseModel};
use reth_db::{tables, TableType, TableViewer};
use std::fmt;

reth_db::tables! {
    /// Store global index
    /// 0 => global index of latest seen
    /// 1 => global index of latest fully processed
    table GlobalIndexTable<Key = u32, Value = u64>;

    /// Requests table, keyed by global index.
    table RequestTable<Key = u64, Value = RequestModel>;
}

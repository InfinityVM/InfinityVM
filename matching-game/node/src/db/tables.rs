//! Tables for the database.

use crate::db::models::{MatchingGameStateModel, RequestModel, ResponseModel};
use reth_db::{tables, TableType, TableViewer};
use std::fmt;

reth_db::tables! {
    /// Store global index
    /// 0 => global index of latest seen
    /// 1 => global index of latest fully processed
    table GlobalIndexTable<Key = u32, Value = u64>;

    /// Requests table, keyed by global index.
    table RequestTable<Key = u64, Value = RequestModel>;

    /// Matching Game State table, keyed by global index.
    table MatchingGameStateTable<Key = u64, Value = MatchingGameStateModel>;
}

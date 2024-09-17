//! Tables for the database.

use crate::db::models::{ClobStateModel, RequestModel, ResponseModel, VecDiffModel};
use reth_db::{tables, TableType, TableViewer};
use std::fmt;

reth_db::tables! {
    /// Store global index
    /// 0 => global index of latest seen
    /// 1 => global index of latest fully processed
    table GlobalIndexTable<Key = u32, Value = u64>;

    /// Requests table, keyed by global index.
    table RequestTable<Key = u64, Value = RequestModel>;

    /// Responses table, keyed by global index.
    table ResponseTable<Key = u64, Value = ResponseModel>;

    /// ClOB State table, keyed by global index.
    table ClobStateTable<Key = u64, Value = ClobStateModel>;

    /// Diff table, keyed by global index.
    table DiffTable<Key = u64, Value = VecDiffModel>;

    table BlockHeightTable<Key = u32, Value = u64>;
}

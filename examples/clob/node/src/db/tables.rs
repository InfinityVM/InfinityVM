//! Tables for the database.

use crate::db::models::{ClobStateModel, RequestModel, ResponseModel, VecDiffModel};
use reth_db::{tables, TableType, TableViewer};
use std::fmt;

reth_db::tables! {
    /// Store global index
    /// 0 => global index of latest seen
    /// 1 => global index of latest fully processed
    table GlobalIndexTable {
       type Key = u32; 
       type Value = u64;
    }

    /// Requests table, keyed by global index.
    table RequestTable {
        type Key = u64; 
        type Value = RequestModel;
    }

    /// Responses table, keyed by global index.
    table ResponseTable {
        type Key = u64; 
        type Value = ResponseModel; 
    }

    /// ClOB State table, keyed by global index.
    table ClobStateTable {
        type Key = u64; 
        type Value = ClobStateModel;
    }

    /// Diff table, keyed by global index.
    table DiffTable {
        type Key = u64; 
        type Value = VecDiffModel; 
    }

    table BlockHeightTable { 
        type Key = u32; 
        type Value = u64; 
    }
}

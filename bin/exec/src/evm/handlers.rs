//! `revm` execution handlers.

use revm::{
    context_interface::{
        result::InvalidHeader, transaction::Eip4844Tx, Block, Transaction, TransactionGetter,
        TransactionType,
    },
    handler::{EthPreExecution, EthPreExecutionContext, EthPreExecutionError},
    handler_interface::PreExecutionHandler,
    precompile::PrecompileErrors,
    primitives::U256,
};

/// Evm PreExecution handler.
pub struct IvmPreExecution<CTX, ERROR> {
    inner: EthPreExecution<CTX, ERROR>,
}

impl<CTX, ERROR> IvmPreExecution<CTX, ERROR> {
    /// Create a new instance of [Self].
    pub fn new() -> Self {
        Self { inner: EthPreExecution::new() }
    }
}

impl<CTX, ERROR> PreExecutionHandler for IvmPreExecution<CTX, ERROR>
where
    CTX: EthPreExecutionContext,
    ERROR: EthPreExecutionError<CTX> + From<InvalidHeader> + From<PrecompileErrors>,
{
    type Context = CTX;
    type Error = ERROR;

    fn load_accounts(&self, context: &mut Self::Context) -> Result<(), Self::Error> {
        self.inner.load_accounts(context)
    }

    fn apply_eip7702_auth_list(&self, context: &mut Self::Context) -> Result<u64, Self::Error> {
        self.inner.apply_eip7702_auth_list(context)
    }

    fn deduct_caller(&self, _context: &mut Self::Context) -> Result<(), Self::Error> {
        // We don't deduct any balance from the caller
        Ok(())
    }
}

//! `revm` execution handlers.

use reth::revm::{
    handler::register::EvmHandler,
    precompile::primitives::{EVMError, InvalidTransaction},
    primitives::EnvWithHandlerCfg,
    Database, Evm, EvmBuilder, GetInspector,
};
use std::{cmp::Ordering, sync::Arc};

/// Handler register that overrides gas payment behavior to not require
/// gas for transactions.
pub fn ivm_gas_handler_register<EXT, DB>(handler: &mut EvmHandler<'_, EXT, DB>)
where
    DB: reth::revm::Database,
{
    handler.pre_execution.deduct_caller = Arc::new(|_ctx| {
        // We don't deduct any balance from the caller because we don't charge gas
        Ok(())
    });

    handler.validation.tx_against_state = Arc::new(|ctx| {
        let caller = ctx.evm.inner.env.tx.caller;
        let state_nonce = ctx
            .evm
            .inner
            .journaled_state
            .load_account(caller, &mut ctx.evm.inner.db)?
            .data
            .info
            .nonce;

        // Check that the transaction's nonce is correct
        if let Some(tx_nonce) = ctx.evm.inner.env.tx.nonce {
            match tx_nonce.cmp(&state_nonce) {
                Ordering::Less => {
                    return Err(EVMError::Transaction(InvalidTransaction::NonceTooLow {
                        tx: tx_nonce,
                        state: state_nonce,
                    }))
                }
                Ordering::Greater => {
                    return Err(EVMError::Transaction(InvalidTransaction::NonceTooHigh {
                        tx: tx_nonce,
                        state: state_nonce,
                    }))
                }
                _ => (/*nonces are equal */),
            }
        }

        Ok(())
    });

    handler.post_execution.refund = Arc::new(|_ctx, _gas, _eip7702_refund| {
        // We can skip refund calculations because we do not reimburse the caller
    });

    handler.post_execution.reimburse_caller = Arc::new(|_ctx, _gas| {
        // No reimbursement because we never deducted gas
        Ok(())
    });

    handler.post_execution.reward_beneficiary = Arc::new(|_ctx, _gas| {
        // Beneficiary does not get rewards because no one paid gas
        Ok(())
    });
}

/// Builder for creating an EVM with a database and environment.
///
/// Wrapper around [`EvmBuilder`] that allows for setting the database and environment for the EVM.
///
/// This is useful for creating an EVM with a custom database and environment without having to
/// necessarily rely on Revm inspector.
///
/// This is based off of the `RethEvmBuilder` with the difference that we register our custom
/// handlers
#[derive(Debug)]
pub struct IvmEvmBuilder<DB: Database, EXT = ()> {
    /// The database to use for the EVM.
    db: DB,
    /// The environment to use for the EVM.
    env: Option<Box<EnvWithHandlerCfg>>,
    /// The external context for the EVM.
    external_context: EXT,
}

impl<DB, EXT> IvmEvmBuilder<DB, EXT>
where
    DB: Database,
{
    /// Create a new EVM builder with the given database.
    pub const fn new(db: DB, external_context: EXT) -> Self {
        Self { db, env: None, external_context }
    }

    /// Set the environment for the EVM.
    pub fn with_env(mut self, env: Box<EnvWithHandlerCfg>) -> Self {
        self.env = Some(env);
        self
    }

    /// Set the external context for the EVM.
    pub fn with_external_context<EXT1>(self, external_context: EXT1) -> IvmEvmBuilder<DB, EXT1> {
        IvmEvmBuilder { db: self.db, env: self.env, external_context }
    }

    /// Build the EVM with the given database and environment.
    pub fn build<'a>(self) -> Evm<'a, EXT, DB> {
        let mut builder = EvmBuilder::default()
            .with_db(self.db)
            .with_external_context(self.external_context)
            .append_handler_register(ivm_gas_handler_register);

        if let Some(env) = self.env {
            builder = builder.with_spec_id(env.clone().spec_id());
            builder = builder.with_env(env.env);
        }

        builder.build()
    }

    /// Build the EVM with the given database and environment, using the given inspector.
    pub fn build_with_inspector<'a, I>(self, inspector: I) -> Evm<'a, I, DB>
    where
        I: GetInspector<DB>,
        EXT: 'a,
    {
        let mut builder =
            EvmBuilder::default().with_db(self.db).with_external_context(self.external_context);

        if let Some(env) = self.env {
            builder = builder.with_spec_id(env.clone().spec_id());
            builder = builder.with_env(env.env);
        }

        builder
            .with_external_context(inspector)
            .append_handler_register(reth::revm::inspector_handle_register)
            .append_handler_register(ivm_gas_handler_register)
            .build()
    }
}

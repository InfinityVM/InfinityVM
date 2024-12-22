//! `revm` execution handlers.

#![allow(missing_debug_implementations)]

use reth::revm::{
    handler::register::EvmHandler,
    precompile::primitives::{EVMError, InvalidTransaction},
};
use std::{cmp::Ordering, sync::Arc};

/// Handler register that overrides gas payment behavior to not require
/// gas for transactions
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

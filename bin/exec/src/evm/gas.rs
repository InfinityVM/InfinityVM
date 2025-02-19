//! Evm builder that includes custom logic for zero gas fee transactions.

use reth_revm::{
    handler::register::EvmHandler,
    precompile::primitives::{EVMError, InvalidTransaction},
    primitives::TxKind,
};
use std::{cmp::Ordering, sync::Arc};

/// Handler register that overrides gas payment behavior to not require
/// gas for transactions.
pub fn ivm_gas_handler_register<EXT, DB>(handler: &mut EvmHandler<'_, EXT, DB>)
where
    DB: reth::revm::Database,
{
    // canonical implementation: https://github.com/bluealloy/revm/blob/900409f134c1cbd4489d370a6b037f354afa4a5c/crates/revm/src/handler/mainnet/pre_execution.rs#L58
    handler.pre_execution.deduct_caller = Arc::new(|ctx| {
        // We don't deduct any balance from the caller because we don't charge gas
        // However, there is a gotcha here: it is expected that the account has
        // its nonce updated and is marked as "touched" here to ensure it gets
        // written to storage.

        let caller = ctx.evm.inner.env.tx.caller;
        let mut caller_account =
            ctx.evm.inner.journaled_state.load_account(caller, &mut ctx.evm.inner.db)?;

        // Bump the nonce for calls. Nonce for CREATE will be bumped in the create logic.
        // See `create_account_checkpoint` in `revm`
        if matches!(ctx.evm.inner.env.tx.transact_to, TxKind::Call(_)) {
            // Nonce is already checked, so this is safe
            caller_account.info.nonce = caller_account.info.nonce.saturating_add(1);
        }

        // touch account so we know it is changed.
        caller_account.mark_touch();

        Ok(())
    });

    // canonical implementation: https://github.com/bluealloy/revm/blob/900409f134c1cbd4489d370a6b037f354afa4a5c/crates/primitives/src/env.rs#L220
    handler.validation.tx_against_state = Arc::new(|ctx| {
        let caller = ctx.evm.inner.env.tx.caller;
        let account =
            ctx.evm.inner.journaled_state.load_account(caller, &mut ctx.evm.inner.db)?.data;

        // EIP-3607: Reject transactions from senders with deployed code
        // Follows logic from here https://github.com/bluealloy/revm/blob/900409f134c1cbd4489d370a6b037f354afa4a5c/crates/primitives/src/env.rs#L228
        if let Some(bytecode) = account.info.code.as_ref() {
            // allow EOAs whose code is a valid delegation designation,
            // i.e. 0xef0100 || address, to continue to originate transactions.
            if !bytecode.is_empty() && !bytecode.is_eip7702() {
                return Err(EVMError::Transaction(InvalidTransaction::RejectCallerWithCode));
            }
        }

        let state_nonce = account.info.nonce;

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
                Ordering::Equal => (/* nonces are equal */),
            }
        }

        Ok(())
    });

    // canonical implementation: https://github.com/bluealloy/revm/blob/900409f134c1cbd4489d370a6b037f354afa4a5c/crates/revm/src/handler/mainnet/post_execution.rs#L59
    handler.post_execution.refund = Arc::new(|_ctx, _gas, _eip7702_refund| {
        // We can skip refund calculations because we do not reimburse the caller
    });

    // canonical implementation: https://github.com/bluealloy/revm/blob/900409f134c1cbd4489d370a6b037f354afa4a5c/crates/revm/src/handler/mainnet/post_execution.rs#L73
    handler.post_execution.reimburse_caller = Arc::new(|_ctx, _gas| {
        // No reimbursement because we never deducted gas
        Ok(())
    });

    // canonical implementation: https://github.com/bluealloy/revm/blob/900409f134c1cbd4489d370a6b037f354afa4a5c/crates/revm/src/handler/mainnet/post_execution.rs#L28
    handler.post_execution.reward_beneficiary = Arc::new(|_ctx, _gas| {
        // Beneficiary does not get rewards because no one paid gas
        Ok(())
    });
}

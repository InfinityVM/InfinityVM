//! `revm` execution handlers.

#![allow(missing_debug_implementations)]

use reth::revm::handler::register::EvmHandler;
use std::{cmp::Ordering, sync::Arc};
use reth::revm::precompile::primitives::EVMError;
use reth::revm::precompile::primitives::InvalidTransaction;

/// Handler register that overrides gas payment behavior to not require
/// gas for transactions
pub fn ivm_gas_handler_register<'a, EXT, DB>(handler: &mut EvmHandler<'a, EXT, DB>)
where
    DB: reth::revm::Database,
{
    // TODO: check default pre-execution load accounts impl

    handler.pre_execution.deduct_caller = Arc::new(|_ctx| {
        // We don't deduct any balance from the caller because we don't charge gas
        Ok(())
    });

    handler.validation.tx_against_state = Arc::new(|ctx| {
        let caller = ctx.evm.inner.env.tx.caller;
        let state_nonce = ctx 
            .evm
            .inner
            .journaled_state.load_account(caller, &mut ctx.evm.inner.db)?.data.info.nonce;

        // Check that the transaction's nonce is correct
        if let Some(tx_nonce) = ctx.evm.inner.env.tx.nonce { 
              match tx_nonce.cmp(&state_nonce) {
                Ordering::Less => {
                    return Err(EVMError::Transaction(InvalidTransaction::NonceTooLow  {
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

    handler.post_execution.refund = Arc::new(|_ctx, _gas, _eip7702_refund|{
g      // We can skip refund calculations because we do not reimburse the caller
    });

    handler.post_execution.reimburse_caller = Arc::new(|_ctx, _gas|{
      // No reimbursement because we never deducted gas
      Ok(())
    });

    handler.post_execution.reward_beneficiary = Arc::new(|_ctx, _gas|{
      // Beneficiary does not get rewards because no one paid gas
      Ok(())
    });
}

// use revm::{
//     handler::{EthExecution, EthHandler},
// };

// pub use crate::evm::handlers::pre::IvmPreExecution;
// pub use crate::evm::handlers::validation::IvmValidation;
// pub use crate::evm::handlers::post::IvmPostExecution;

// /// Evm handler
// pub type IvmHandler<
//     CTX,
//     ERROR,
//     VAL = IvmValidation<CTX, ERROR>,
//     PREEXEC = IvmPreExecution<CTX, ERROR>,
//     EXEC = EthExecution<CTX, ERROR>,
//     POSTEXEC = IvmPostExecution<CTX, ERROR>,
// > = EthHandler<CTX, ERROR, VAL, PREEXEC, EXEC, POSTEXEC>;

// /// Pre execution handler
// pub mod pre {
//     use revm::{
//         context_interface::result::InvalidHeader,
//         handler::{EthPreExecution, EthPreExecutionContext, EthPreExecutionError},
//         handler_interface::PreExecutionHandler,
//         precompile::PrecompileErrors,
//     };

//     /// Evm `PreExecution` handler.
//     pub struct IvmPreExecution<CTX, ERROR> {
//         eth: EthPreExecution<CTX, ERROR>,
//     }

//     impl<CTX, ERROR> Default for IvmPreExecution<CTX, ERROR> {
//         fn default() -> Self {
//             Self::new()
//         }
//     }

//     impl<CTX, ERROR> IvmPreExecution<CTX, ERROR> {
//         /// Create a new instance of [Self].
//         pub fn new() -> Self {
//             Self { eth: EthPreExecution::new() }
//         }
//     }

//     impl<CTX, ERROR> PreExecutionHandler for IvmPreExecution<CTX, ERROR>
//     where
//         CTX: EthPreExecutionContext,
//         ERROR: EthPreExecutionError<CTX> + From<InvalidHeader> + From<PrecompileErrors>,
//     {
//         type Context = CTX;
//         type Error = ERROR;

//         fn load_accounts(&self, context: &mut Self::Context) -> Result<(), Self::Error> {
//             self.eth.load_accounts(context)
//         }

//         fn apply_eip7702_auth_list(&self, context: &mut Self::Context) -> Result<u64,
// Self::Error> {             self.eth.apply_eip7702_auth_list(context)
//         }

//         fn deduct_caller(&self, _context: &mut Self::Context) -> Result<(), Self::Error> {
//             // We don't deduct any balance from the caller because we don't charge gas
//             Ok(())
//         }
//     }
// }

// /// Transaction validation handler
// pub mod validation {
//     use revm::{
//         context::Cfg,
//         context_interface::{
//             result::InvalidTransaction, Transaction,
//             TransactionGetter,

//             // JournaledState,
//         },
//         handler::{EthValidation, EthValidationContext, EthValidationError},
//         handler_interface::ValidationHandler,
//     };
//     use revm::context_interface::Journal;
//     use std::cmp::Ordering;

//     /// Evm validation handler.
//     pub struct IvmValidation<CTX, ERROR> {
//         eth: EthValidation<CTX, ERROR>,
//     }

//     impl<CTX, ERROR> Default for IvmValidation<CTX, ERROR> {
//         fn default() -> Self {
//             Self::new()
//         }
//     }

//     impl<CTX, ERROR> IvmValidation<CTX, ERROR> {
//         /// Create a new instance of [Self]
//         pub fn new() -> Self {
//             Self { eth: EthValidation::new() }
//         }
//     }

//     impl<CTX, ERROR> ValidationHandler for IvmValidation<CTX, ERROR>
//     where
//         CTX: EthValidationContext,
//         ERROR: EthValidationError<CTX>,
//     {
//         type Context = CTX;
//         type Error = ERROR;

//         fn validate_env(&self, context: &Self::Context) -> Result<(), Self::Error> {
//             self.eth.validate_env(context)
//         }

//         fn validate_tx_against_state(
//             &self,
//             context: &mut Self::Context,
//         ) -> Result<(), Self::Error> {
//             let caller = context.tx().common_fields().caller();
//             let caller_nonce = context.journal().load_account(caller)?.data.info.nonce;

//             if !context.cfg().is_nonce_check_disabled() {
//                 let tx_nonce = context.tx().common_fields().nonce();
//                 let state_nonce = caller_nonce;
//                 match tx_nonce.cmp(&state_nonce) {
//                     Ordering::Less => {
//                         return Err(ERROR::from(InvalidTransaction::NonceTooLow {
//                             tx: tx_nonce,
//                             state: state_nonce,
//                         }))
//                     }
//                     Ordering::Greater => {
//                         return Err(ERROR::from(InvalidTransaction::NonceTooHigh {
//                             tx: tx_nonce,
//                             state: state_nonce,
//                         }))
//                     }
//                     _ => (),
//                 }
//             }

//             Ok(())
//         }

//         fn validate_initial_tx_gas(&self, context: &Self::Context) -> Result<u64, Self::Error> {
//             self.eth.validate_initial_tx_gas(context)
//         }
//     }
// }

// /// Post execution handler
// pub mod post {
//     use revm::context_interface::result::{HaltReasonTrait, InvalidHeader, InvalidTransaction};
//     use revm::context_interface::JournalStateGetterDBError;
//     use revm::handler::{EthPostExecutionContext, EthPostExecutionError};
//     use revm::precompile::PrecompileErrors;
//     use revm::{
//         context_interface::result::{HaltReason, ResultAndState},
//         handler::{EthPostExecution, FrameResult},
//         handler_interface::PostExecutionHandler,
//     };

//     /// Evm post execution handler.
//     pub struct IvmPostExecution<CTX, ERROR, HALTREASON = HaltReason> {
//         eth: EthPostExecution<CTX, ERROR, HALTREASON>,
//     }

//     impl<CTX, ERROR, HALTREASON> Default for IvmPostExecution<CTX, ERROR, HALTREASON> {
//         fn default() -> Self {
//             Self::new()
//         }
//     }

//     impl<CTX, ERROR, HALTREASON> IvmPostExecution<CTX, ERROR, HALTREASON> {
//         /// Create a new instance of [Self]
//         pub fn new() -> Self {
//             Self {
//                 eth: EthPostExecution::new(),
//             }
//         }
//     }

//     impl<CTX, ERROR, HALTREASON> PostExecutionHandler for IvmPostExecution<CTX, ERROR,
// HALTREASON>     where
//         CTX: EthPostExecutionContext<ERROR>,
//         ERROR: EthPostExecutionError<CTX>
//             + From<InvalidTransaction>
//             + From<InvalidHeader>
//             + From<JournalStateGetterDBError<CTX>>
//             + From<PrecompileErrors>,
//         HALTREASON: HaltReasonTrait,
//     {
//         type Context = CTX;
//         type Error = ERROR;
//         type ExecResult = FrameResult;
//         type Output = ResultAndState<HALTREASON>;

//         fn refund(
//             &self,
//             context: &mut Self::Context,
//             exec_result: &mut Self::ExecResult,
//             eip7702_refund: i64,
//         ) {
//             // TODO: read through refund code
//             self.eth.refund(context, exec_result, eip7702_refund)
//         }

//         fn reimburse_caller(
//             &self,
//             _context: &mut Self::Context,
//             _exec_result: &mut Self::ExecResult,
//         ) -> Result<(), Self::Error> {

//             Ok(())
//         }

//         fn reward_beneficiary(
//             &self,
//             _context: &mut Self::Context,
//             _exec_result: &mut Self::ExecResult,
//         ) -> Result<(), Self::Error> {

//             Ok(())
//         }

//         fn output(
//             &self,
//             context: &mut Self::Context,
//             result: Self::ExecResult,
//         ) -> Result<Self::Output, Self::Error> {
//             self.eth.output(context, result)
//         }

//         fn clear(&self, context: &mut Self::Context) {
//             self.eth.clear(context)
//         }
//     }

// }

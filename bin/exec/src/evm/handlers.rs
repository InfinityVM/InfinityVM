//! `revm` execution handlers.

#![allow(missing_debug_implementations)]

/// Pre execution handler
pub mod pre {
    use revm::{
        context_interface::result::InvalidHeader,
        handler::{EthPreExecution, EthPreExecutionContext, EthPreExecutionError},
        handler_interface::PreExecutionHandler,
        precompile::PrecompileErrors,
    };

    /// Evm `PreExecution` handler.
    pub struct IvmPreExecution<CTX, ERROR> {
        eth: EthPreExecution<CTX, ERROR>,
    }

    impl<CTX, ERROR> Default for IvmPreExecution<CTX, ERROR> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<CTX, ERROR> IvmPreExecution<CTX, ERROR> {
        /// Create a new instance of [Self].
        pub fn new() -> Self {
            Self { eth: EthPreExecution::new() }
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
            self.eth.load_accounts(context)
        }

        fn apply_eip7702_auth_list(&self, context: &mut Self::Context) -> Result<u64, Self::Error> {
            self.eth.apply_eip7702_auth_list(context)
        }

        fn deduct_caller(&self, _context: &mut Self::Context) -> Result<(), Self::Error> {
            // We don't deduct any balance from the caller because we don't charge gas
            Ok(())
        }
    }
}

///Post execution handler
pub mod validation {
    use revm::{
        context::Cfg,
        context_interface::{
            result::InvalidTransaction, Transaction,
            TransactionGetter,

            // JournaledState, 
        },
        handler::{EthValidation, EthValidationContext, EthValidationError},
        handler_interface::ValidationHandler,
    };
    use revm::context_interface::Journal;
    use std::cmp::Ordering;

    /// Evm validation handler.
    pub struct IvmValidation<CTX, ERROR> {
        eth: EthValidation<CTX, ERROR>,
    }

    impl<CTX, ERROR> Default for IvmValidation<CTX, ERROR> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<CTX, ERROR> IvmValidation<CTX, ERROR> {
        /// Create a new instance of [Self]
        pub fn new() -> Self {
            Self { eth: EthValidation::new() }
        }
    }

    impl<CTX, ERROR> ValidationHandler for IvmValidation<CTX, ERROR>
    where
        CTX: EthValidationContext,
        ERROR: EthValidationError<CTX>,
    {
        type Context = CTX;
        type Error = ERROR;

        fn validate_env(&self, context: &Self::Context) -> Result<(), Self::Error> {
            self.eth.validate_env(context)
        }

        fn validate_tx_against_state(
            &self,
            context: &mut Self::Context,
        ) -> Result<(), Self::Error> {
            let caller = context.tx().common_fields().caller();
            let caller_nonce = context.journal().load_account(caller)?.data.info.nonce;

            if !context.cfg().is_nonce_check_disabled() {
                let tx_nonce = context.tx().common_fields().nonce();
                let state_nonce = caller_nonce;
                match tx_nonce.cmp(&state_nonce) {
                    Ordering::Less => {
                        return Err(ERROR::from(InvalidTransaction::NonceTooLow {
                            tx: tx_nonce,
                            state: state_nonce,
                        }))
                    }
                    Ordering::Greater => {
                        return Err(ERROR::from(InvalidTransaction::NonceTooHigh {
                            tx: tx_nonce,
                            state: state_nonce,
                        }))
                    }
                    _ => (),
                }
            }

            Ok(())
        }

        fn validate_initial_tx_gas(&self, context: &Self::Context) -> Result<u64, Self::Error> {
            self.eth.validate_initial_tx_gas(context)
        }
    }
}

pub mod post {
    use revm::context_interface::result::{HaltReasonTrait, InvalidHeader, InvalidTransaction};
    use revm::context_interface::JournalStateGetterDBError;
    use revm::handler::{EthPostExecutionContext, EthPostExecutionError};
    use revm::precompile::PrecompileErrors;
    use revm::{
        context_interface::result::{HaltReason, ResultAndState},
        handler::{EthPostExecution, FrameResult},
        handler_interface::PostExecutionHandler,
    };

    /// Evm post execution handler.
    
    pub struct IvmPostExecution<CTX, ERROR, HALTREASON = HaltReason> {
        eth: EthPostExecution<CTX, ERROR, HALTREASON>,
    }

    impl<CTX, ERROR, HALTREASON> Default for IvmPostExecution<CTX, ERROR, HALTREASON> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<CTX, ERROR, HALTREASON> IvmPostExecution<CTX, ERROR, HALTREASON> {
        pub fn new() -> Self {
            Self {
                eth: EthPostExecution::new(),
            }
        }
    }

    impl<CTX, ERROR, HALTREASON> PostExecutionHandler for IvmPostExecution<CTX, ERROR, HALTREASON>
    where
        CTX: EthPostExecutionContext<ERROR>,
        ERROR: EthPostExecutionError<CTX>
            + From<InvalidTransaction>
            + From<InvalidHeader>
            + From<JournalStateGetterDBError<CTX>>
            + From<PrecompileErrors>,
        HALTREASON: HaltReasonTrait,
    {
        type Context = CTX;
        type Error = ERROR;
        type ExecResult = FrameResult;
        type Output = ResultAndState<HALTREASON>;

        fn refund(
            &self,
            context: &mut Self::Context,
            exec_result: &mut Self::ExecResult,
            eip7702_refund: i64,
        ) {
            self.eth.refund(context, exec_result, eip7702_refund)
        }

        fn reimburse_caller(
            &self,
            context: &mut Self::Context,
            exec_result: &mut Self::ExecResult,
        ) -> Result<(), Self::Error> {
            // let basefee = context.block().basefee();
            // let caller = context.tx().common_fields().caller();
            // let effective_gas_price = context.tx().effective_gas_price(*basefee);
            // let gas = exec_result.gas();

            // let reimbursement =
            //     effective_gas_price * U256::from(gas.remaining() + gas.refunded() as u64);
            // token_operation::<CTX, ERROR>(context, TREASURY, caller, reimbursement)?;

            Ok(())
        }

        fn reward_beneficiary(
            &self,
            context: &mut Self::Context,
            exec_result: &mut Self::ExecResult,
        ) -> Result<(), Self::Error> {
            // let tx = context.tx();
            // let beneficiary = context.block().beneficiary();
            // let basefee = context.block().basefee();
            // let effective_gas_price = tx.effective_gas_price(*basefee);
            // let gas = exec_result.gas();

            // let coinbase_gas_price = if context.cfg().spec().into().is_enabled_in(SpecId::LONDON) {
            //     effective_gas_price.saturating_sub(*basefee)
            // } else {
            //     effective_gas_price
            // };

            // let reward = coinbase_gas_price * U256::from(gas.spent() - gas.refunded() as u64);
            // token_operation::<CTX, ERROR>(context, TREASURY, *beneficiary, reward)?;

            Ok(())
        }

        fn output(
            &self,
            context: &mut Self::Context,
            result: Self::ExecResult,
        ) -> Result<Self::Output, Self::Error> {
            self.eth.output(context, result)
        }

        fn clear(&self, context: &mut Self::Context) {
            self.eth.clear(context)
        }
    }

}
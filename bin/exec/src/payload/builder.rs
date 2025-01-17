//! Payload builder.

use std::sync::Arc;

use alloy_consensus::Header;
use reth_chainspec::ChainSpec;
use reth_ethereum_engine_primitives::{EthBuiltPayload, EthPayloadBuilderAttributes};
use reth_ethereum_payload_builder::{default_ethereum_payload, EthereumBuilderConfig};
use reth_evm::{env::EvmEnv, ConfigureEvm, NextBlockEnvAttributes};
use reth_primitives::TransactionSigned;
use reth_provider::{ChainSpecProvider, StateProviderFactory};
use reth_transaction_pool::{noop::NoopTransactionPool, BestTransactions, PoolTransaction, TransactionPool, ValidPoolTransaction};
use reth_basic_payload_builder::{BuildArguments, BuildOutcome, PayloadConfig};
use reth_node_api::{PayloadBuilderAttributes, PayloadBuilderError};
use reth_basic_payload_builder::PayloadBuilder;

use crate::evm::IvmEvmConfig;

type BestTransactionsIter<Pool> = Box<
    dyn BestTransactions<Item = Arc<ValidPoolTransaction<<Pool as TransactionPool>::Transaction>>>,
>;

/// Ethereum payload builder
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IvmPayloadBuilder<EvmConfig = IvmEvmConfig> {
    /// The type responsible for creating the evm.
    evm_config: EvmConfig,
    /// Payload builder configuration.
    builder_config: EthereumBuilderConfig,
}

impl<EvmConfig> IvmPayloadBuilder<EvmConfig> {
    /// `IvmPayloadBuilder` constructor.
    pub const fn new(evm_config: EvmConfig, builder_config: EthereumBuilderConfig) -> Self {
        Self { evm_config, builder_config }
    }
}

impl<EvmConfig> IvmPayloadBuilder<EvmConfig>
where
    EvmConfig: ConfigureEvm<Header = Header>,
{
    /// Returns the configured [`CfgEnvWithHandlerCfg`] and [`BlockEnv`] for the targeted payload
    /// (that has the `parent` as its parent).
    fn cfg_and_block_env(
        &self,
        config: &PayloadConfig<EthPayloadBuilderAttributes>,
        parent: &Header,
    ) -> Result<EvmEnv, EvmConfig::Error> {
        let next_attributes = NextBlockEnvAttributes {
            timestamp: config.attributes.timestamp(),
            suggested_fee_recipient: config.attributes.suggested_fee_recipient(),
            prev_randao: config.attributes.prev_randao(),
            gas_limit: self.builder_config.gas_limit(parent.gas_limit),
        };
        self.evm_config.next_cfg_and_block_env(parent, next_attributes)
    }
}

// Default implementation of [PayloadBuilder] for unit type
impl<EvmConfig, Pool, Client> PayloadBuilder<Pool, Client> for IvmPayloadBuilder<EvmConfig>
where
    EvmConfig: ConfigureEvm<Header = Header, Transaction = TransactionSigned>,
    Client: StateProviderFactory + ChainSpecProvider<ChainSpec = ChainSpec>,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TransactionSigned>>,
{
    type Attributes = EthPayloadBuilderAttributes;
    type BuiltPayload = EthBuiltPayload;

    fn try_build(
        &self,
        args: BuildArguments<Pool, Client, EthPayloadBuilderAttributes, EthBuiltPayload>,
    ) -> Result<BuildOutcome<EthBuiltPayload>, PayloadBuilderError> {
        let EvmEnv { cfg_env_with_handler_cfg, block_env } = self
            .cfg_and_block_env(&args.config, &args.config.parent_header)
            .map_err(PayloadBuilderError::other)?;

        let pool = args.pool.clone();
        default_ethereum_payload(
            self.evm_config.clone(),
            self.builder_config.clone(),
            args,
            cfg_env_with_handler_cfg,
            block_env,
            |attributes| pool.best_transactions_with_attributes(attributes),
        )
    }

    fn build_empty_payload(
        &self,
        client: &Client,
        config: PayloadConfig<Self::Attributes>,
    ) -> Result<EthBuiltPayload, PayloadBuilderError> {
        let args = BuildArguments::new(
            client,
            // we use defaults here because for the empty payload we don't need to execute anything
            NoopTransactionPool::default(),
            Default::default(),
            config,
            Default::default(),
            None,
        );

        let EvmEnv { cfg_env_with_handler_cfg, block_env } = self
            .cfg_and_block_env(&args.config, &args.config.parent_header)
            .map_err(PayloadBuilderError::other)?;

        let pool = args.pool.clone();

        default_ethereum_payload(
            self.evm_config.clone(),
            self.builder_config.clone(),
            args,
            cfg_env_with_handler_cfg,
            block_env,
            |attributes| pool.best_transactions_with_attributes(attributes),
        )?
        .into_payload()
        .ok_or_else(|| PayloadBuilderError::MissingPayload)
    }
}

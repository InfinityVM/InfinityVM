//! IVM transaction pool. The primary difference from the default transaction pool is
//! additional validation logic.

use crate::{config::IvmConfig, pool::validator::IvmTransactionValidator};
use reth_chainspec::ChainSpec;
use reth_node_api::NodeTypes;
use reth_node_builder::{components::PoolBuilder, BuilderContext, FullNodeTypes};
use reth_primitives::EthPrimitives;
use reth_provider::CanonStateSubscriptions;
use reth_transaction_pool::{
    blobstore::DiskFileBlobStore, CoinbaseTipOrdering, EthPooledTransaction, PoolTransaction,
    Priority, TransactionOrdering, TransactionValidationTaskExecutor,
};
use tracing::{debug, info};
use validator::IvmTransactionValidatorBuilder;

pub mod validator;

/// Gasless ordering prioritizes any senders that are explicitly allowed. All explicitly allowed
/// senders are prioritized equally. And all others are prioritized equally.
///
/// The way this works in practice rests heavily on the implementation of the reth pool components.
/// Based on the reth pool docs we expect that if any two transactions have the same priority score,
/// then the transaction in the pool longer takes precedence.
///
/// Based on a close reading of reth logic, a major caveat is that transactions
/// with a base fee lower then the current base fee will not be considered. See
/// [`best_with_basefee_and_blobfee`](https://github.com/InfinityVM/reth/blob/28d52312acd46be2bfc46661a7b392feaa2bd4c5/crates/transaction-pool/src/pool/pending.rs#L112`).
///
/// Below are some details on the reth code that interacts with prioritizing:
///
/// The way priority works in the reth pool is that all pending transactions are stored with their
/// "Priority":
///
/// ```compile_fail
/// /// A transaction that is ready to be included in a block.
/// #[derive(Debug)]
/// pub(crate) struct PendingTransaction<T: TransactionOrdering> {
///     /// Identifier that tags when transaction was submitted in the pool.
///     pub(crate) submission_id: u64,
///     /// Actual transaction.
///     pub(crate) transaction: Arc<ValidPoolTransaction<T::Transaction>>,
///     /// The priority value assigned by the used `Ordering` function.
///     pub(crate) priority: Priority<T::PriorityValue>,
/// }
/// ```
///
/// When we go to get the "best" transactions, we collect all the transactions that can be included
/// right now (are not dependent on other transactions) into a `BTreeSet`. Their ordering in the
/// `BTreeSet` is dictated by their priority (`TransactionOrdering`):
/// ```compile_fail
/// // As defined on PendingPool<T>
/// pub(crate) fn best(&self) -> BestTransactions<T> {
///     BestTransactions {
///         all: self.by_id.clone(),
///         // Collect into BTreeSet
///         independent: self.independent_transactions.values().cloned().collect(),
///         invalid: Default::default(),
///         new_transaction_receiver: Some(self.new_transaction_notifier.subscribe()),
///         skip_blobs: false,
///     }
/// }
/// ```
/// Importantly, the docs for `best` note that: "If two transactions have the same priority score,
/// then the transactions which spent more time in pool (were added earlier) are returned first."
/// See: <https://github.com/InfinityVM/reth/blob/28d52312acd46be2bfc46661a7b392feaa2bd4c5/crates/transaction-pool/src/pool/pending.rs#L93>
///
/// We can see this is due to the `Ord` impl on `PendingTransaction`
///
/// ```compile_fail
/// impl<T: TransactionOrdering> Ord for PendingTransaction<T> {
///    fn cmp(&self, other: &Self) -> Ordering {
///        // This compares by `priority` and only if two tx have the exact same priority this compares
///        // the unique `submission_id`. This ensures that transactions with same priority are not
///        // equal, so they're not replaced in the set
///       self.priority
///           .cmp(&other.priority)
///            .then_with(|| other.submission_id.cmp(&self.submission_id))
///    }
/// }
/// ```
/// See <https://github.com/InfinityVM/reth/blob/28d52312acd46be2bfc46661a7b392feaa2bd4c5/crates/transaction-pool/src/pool/pending.rs#L592-L601>
#[derive(Debug, Default)]
pub struct GaslessOrdering {
    ivm_config: IvmConfig,
}

impl GaslessOrdering {
    /// Create a new instance of `GaslessOrdering`
    pub const fn new(ivm_config: IvmConfig) -> Self {
        Self { ivm_config }
    }
}

impl TransactionOrdering for GaslessOrdering {
    type PriorityValue = u32;
    type Transaction = EthPooledTransaction;

    /// Higher is better.
    fn priority(
        &self,
        transaction: &Self::Transaction,
        _base_fee: u64,
    ) -> Priority<Self::PriorityValue> {
        let sender = transaction.sender();
        if self.ivm_config.is_priority_sender(&sender) {
            Priority::Value(1)
        } else {
            Priority::Value(0)
        }
    }
}

/// Type describing the IVM transaction pool.
pub type IvmTransactionPool<Client, S> = reth_transaction_pool::Pool<
    TransactionValidationTaskExecutor<IvmTransactionValidator<Client, EthPooledTransaction>>,
    CoinbaseTipOrdering<EthPooledTransaction>,
    S,
>;

/// IVM transaction pool builder
#[derive(Debug, Clone, Default)]
pub struct IvmPoolBuilder {
    ivm_config: IvmConfig,
}

impl IvmPoolBuilder {
    /// Create a new [`IvmPoolBuilder`].
    pub const fn new(ivm_config: IvmConfig) -> Self {
        Self { ivm_config }
    }
}

/// Implement the [`PoolBuilder`] trait for the custom pool builder
///
/// This will be used to build the transaction pool and its maintenance tasks during launch.
impl<Node> PoolBuilder<Node> for IvmPoolBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec, Primitives = EthPrimitives>>,
{
    type Pool = IvmTransactionPool<Node::Provider, DiskFileBlobStore>;

    async fn build_pool(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::Pool> {
        let data_dir = ctx.config().datadir();
        let pool_config = ctx.pool_config();
        let ivm_config: IvmConfig = self.ivm_config;
        let blob_store = DiskFileBlobStore::open(data_dir.blobstore(), Default::default())?;

        let txn_validator = IvmTransactionValidatorBuilder::new(ctx.chain_spec())
            .with_head_timestamp(ctx.head().timestamp)
            .kzg_settings(ctx.kzg_settings()?)
            .with_local_transactions_config(pool_config.local_transactions_config.clone())
            .with_additional_tasks(ctx.config().txpool.additional_validation_tasks)
            .build_with_tasks(
                ctx.provider().clone(),
                ctx.task_executor().clone(),
                blob_store.clone(),
                ivm_config,
            );

        let transaction_pool = reth_transaction_pool::Pool::new(
            txn_validator,
            CoinbaseTipOrdering::default(),
            blob_store,
            pool_config,
        );

        info!(target: "reth::cli", "Transaction pool initialized");
        let transactions_path = data_dir.txpool_transactions();

        // spawn txpool maintenance task
        {
            let pool = transaction_pool.clone();
            let chain_events = ctx.provider().canonical_state_stream();
            let client = ctx.provider().clone();
            let transactions_backup_config =
                reth_transaction_pool::maintain::LocalTransactionBackupConfig::with_local_txs_backup(transactions_path);

            ctx.task_executor().spawn_critical_with_graceful_shutdown_signal(
                "local transactions backup task",
                |shutdown| {
                    reth_transaction_pool::maintain::backup_local_transactions_task(
                        shutdown,
                        pool.clone(),
                        transactions_backup_config,
                    )
                },
            );

            // spawn the maintenance task
            ctx.task_executor().spawn_critical(
                "txpool maintenance task",
                reth_transaction_pool::maintain::maintain_transaction_pool_future(
                    client,
                    pool,
                    chain_events,
                    ctx.task_executor().clone(),
                    Default::default(),
                ),
            );
            debug!(target: "reth::cli", "Spawned txpool maintenance task");
        }

        Ok(transaction_pool)
    }
}

#[cfg(test)]
mod test {
    use crate::config::transaction::IvmTransactionAllowConfig;

    use super::*;
    use alloy_consensus::Transaction;
    use alloy_eips::eip2718::Decodable2718;
    use alloy_primitives::{hex, U256};
    use reth_chainspec::MAINNET;
    use reth_primitives::{transaction::SignedTransactionIntoRecoveredExt, PooledTransaction};
    use reth_provider::{
        test_utils::{ExtendedAccount, MockEthProvider},
        StateProvider,
    };
    use reth_transaction_pool::{
        blobstore::InMemoryBlobStore, error::InvalidPoolTransactionError, EthPooledTransaction,
        Pool, PoolTransaction, TransactionOrigin, TransactionPool, TransactionValidationOutcome,
    };

    fn get_create_transaction() -> EthPooledTransaction {
        // This is taken from the reth transaction pool tests
        let raw = "0x02f914950181ad84b2d05e0085117553845b830f7df88080b9143a6040608081523462000414576200133a803803806200001e8162000419565b9283398101608082820312620004145781516001600160401b03908181116200041457826200004f9185016200043f565b92602092838201519083821162000414576200006d9183016200043f565b8186015190946001600160a01b03821692909183900362000414576060015190805193808511620003145760038054956001938488811c9816801562000409575b89891014620003f3578190601f988981116200039d575b50899089831160011462000336576000926200032a575b505060001982841b1c191690841b1781555b8751918211620003145760049788548481811c9116801562000309575b89821014620002f457878111620002a9575b5087908784116001146200023e5793839491849260009562000232575b50501b92600019911b1c19161785555b6005556007805460ff60a01b19169055600880546001600160a01b0319169190911790553015620001f3575060025469d3c21bcecceda100000092838201809211620001de57506000917fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef9160025530835282815284832084815401905584519384523093a351610e889081620004b28239f35b601190634e487b7160e01b6000525260246000fd5b90606493519262461bcd60e51b845283015260248201527f45524332303a206d696e7420746f20746865207a65726f2061646472657373006044820152fd5b0151935038806200013a565b9190601f198416928a600052848a6000209460005b8c8983831062000291575050501062000276575b50505050811b0185556200014a565b01519060f884600019921b161c191690553880808062000267565b86860151895590970196948501948893500162000253565b89600052886000208880860160051c8201928b8710620002ea575b0160051c019085905b828110620002dd5750506200011d565b60008155018590620002cd565b92508192620002c4565b60228a634e487b7160e01b6000525260246000fd5b90607f16906200010b565b634e487b7160e01b600052604160045260246000fd5b015190503880620000dc565b90869350601f19831691856000528b6000209260005b8d8282106200038657505084116200036d575b505050811b018155620000ee565b015160001983861b60f8161c191690553880806200035f565b8385015186558a979095019493840193016200034c565b90915083600052896000208980850160051c8201928c8610620003e9575b918891869594930160051c01915b828110620003d9575050620000c5565b60008155859450889101620003c9565b92508192620003bb565b634e487b7160e01b600052602260045260246000fd5b97607f1697620000ae565b600080fd5b6040519190601f01601f191682016001600160401b038111838210176200031457604052565b919080601f84011215620004145782516001600160401b038111620003145760209062000475601f8201601f1916830162000419565b92818452828287010111620004145760005b8181106200049d57508260009394955001015290565b85810183015184820184015282016200048756fe608060408181526004918236101561001657600080fd5b600092833560e01c91826306fdde0314610a1c57508163095ea7b3146109f257816318160ddd146109d35781631b4c84d2146109ac57816323b872dd14610833578163313ce5671461081757816339509351146107c357816370a082311461078c578163715018a6146107685781638124f7ac146107495781638da5cb5b1461072057816395d89b411461061d578163a457c2d714610575578163a9059cbb146104e4578163c9567bf914610120575063dd62ed3e146100d557600080fd5b3461011c578060031936011261011c57806020926100f1610b5a565b6100f9610b75565b6001600160a01b0391821683526001865283832091168252845220549051908152f35b5080fd5b905082600319360112610338576008546001600160a01b039190821633036104975760079283549160ff8360a01c1661045557737a250d5630b4cf539739df2c5dacb4c659f2488d92836bffffffffffffffffffffffff60a01b8092161786553087526020938785528388205430156104065730895260018652848920828a52865280858a205584519081527f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925863092a38554835163c45a015560e01b815290861685828581845afa9182156103dd57849187918b946103e7575b5086516315ab88c960e31b815292839182905afa9081156103dd576044879289928c916103c0575b508b83895196879586946364e329cb60e11b8652308c870152166024850152165af19081156103b6579086918991610389575b50169060065416176006558385541660604730895288865260c4858a20548860085416928751958694859363f305d71960e01b8552308a86015260248501528d60448501528d606485015260848401524260a48401525af1801561037f579084929161034c575b50604485600654169587541691888551978894859363095ea7b360e01b855284015260001960248401525af1908115610343575061030c575b5050805460ff60a01b1916600160a01b17905580f35b81813d831161033c575b6103208183610b8b565b8101031261033857518015150361011c5738806102f6565b8280fd5b503d610316565b513d86823e3d90fd5b6060809293503d8111610378575b6103648183610b8b565b81010312610374578290386102bd565b8580fd5b503d61035a565b83513d89823e3d90fd5b6103a99150863d88116103af575b6103a18183610b8b565b810190610e33565b38610256565b503d610397565b84513d8a823e3d90fd5b6103d79150843d86116103af576103a18183610b8b565b38610223565b85513d8b823e3d90fd5b6103ff919450823d84116103af576103a18183610b8b565b92386101fb565b845162461bcd60e51b81528085018790526024808201527f45524332303a20617070726f76652066726f6d20746865207a65726f206164646044820152637265737360e01b6064820152608490fd5b6020606492519162461bcd60e51b8352820152601760248201527f74726164696e6720697320616c7265616479206f70656e0000000000000000006044820152fd5b608490602084519162461bcd60e51b8352820152602160248201527f4f6e6c79206f776e65722063616e2063616c6c20746869732066756e6374696f6044820152603760f91b6064820152fd5b9050346103385781600319360112610338576104fe610b5a565b9060243593303303610520575b602084610519878633610bc3565b5160018152f35b600594919454808302908382041483151715610562576127109004820391821161054f5750925080602061050b565b634e487b7160e01b815260118552602490fd5b634e487b7160e01b825260118652602482fd5b9050823461061a578260031936011261061a57610590610b5a565b918360243592338152600160205281812060018060a01b03861682526020522054908282106105c9576020856105198585038733610d31565b608490602086519162461bcd60e51b8352820152602560248201527f45524332303a2064656372656173656420616c6c6f77616e63652062656c6f77604482015264207a65726f60d81b6064820152fd5b80fd5b83833461011c578160031936011261011c57805191809380549160019083821c92828516948515610716575b6020958686108114610703578589529081156106df5750600114610687575b6106838787610679828c0383610b8b565b5191829182610b11565b0390f35b81529295507f8a35acfbc15ff81a39ae7d344fd709f28e8600b4aa8c65c6b64bfe7fe36bd19b5b8284106106cc57505050826106839461067992820101948680610668565b80548685018801529286019281016106ae565b60ff19168887015250505050151560051b8301019250610679826106838680610668565b634e487b7160e01b845260228352602484fd5b93607f1693610649565b50503461011c578160031936011261011c5760085490516001600160a01b039091168152602090f35b50503461011c578160031936011261011c576020906005549051908152f35b833461061a578060031936011261061a57600880546001600160a01b031916905580f35b50503461011c57602036600319011261011c5760209181906001600160a01b036107b4610b5a565b16815280845220549051908152f35b82843461061a578160031936011261061a576107dd610b5a565b338252600160209081528383206001600160a01b038316845290528282205460243581019290831061054f57602084610519858533610d31565b50503461011c578160031936011261011c576020905160128152f35b83833461011c57606036600319011261011c5761084e610b5a565b610856610b75565b6044359160018060a01b0381169485815260209560018752858220338352875285822054976000198903610893575b505050906105199291610bc3565b85891061096957811561091a5733156108cc5750948481979861051997845260018a528284203385528a52039120558594938780610885565b865162461bcd60e51b8152908101889052602260248201527f45524332303a20617070726f766520746f20746865207a65726f206164647265604482015261737360f01b6064820152608490fd5b865162461bcd60e51b81529081018890526024808201527f45524332303a20617070726f76652066726f6d20746865207a65726f206164646044820152637265737360e01b6064820152608490fd5b865162461bcd60e51b8152908101889052601d60248201527f45524332303a20696e73756666696369656e7420616c6c6f77616e63650000006044820152606490fd5b50503461011c578160031936011261011c5760209060ff60075460a01c1690519015158152f35b50503461011c578160031936011261011c576020906002549051908152f35b50503461011c578060031936011261011c57602090610519610a12610b5a565b6024359033610d31565b92915034610b0d5783600319360112610b0d57600354600181811c9186908281168015610b03575b6020958686108214610af05750848852908115610ace5750600114610a75575b6106838686610679828b0383610b8b565b929550600383527fc2575a0e9e593c00f959f8c92f12db2869c3395a3b0502d05e2516446f71f85b5b828410610abb575050508261068394610679928201019438610a64565b8054868501880152928601928101610a9e565b60ff191687860152505050151560051b83010192506106798261068338610a64565b634e487b7160e01b845260229052602483fd5b93607f1693610a44565b8380fd5b6020808252825181830181905290939260005b828110610b4657505060409293506000838284010152601f8019910116010190565b818101860151848201604001528501610b24565b600435906001600160a01b0382168203610b7057565b600080fd5b602435906001600160a01b0382168203610b7057565b90601f8019910116810190811067ffffffffffffffff821117610bad57604052565b634e487b7160e01b600052604160045260246000fd5b6001600160a01b03908116918215610cde5716918215610c8d57600082815280602052604081205491808310610c3957604082827fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef958760209652828652038282205586815220818154019055604051908152a3565b60405162461bcd60e51b815260206004820152602660248201527f45524332303a207472616e7366657220616d6f756e7420657863656564732062604482015265616c616e636560d01b6064820152608490fd5b60405162461bcd60e51b815260206004820152602360248201527f45524332303a207472616e7366657220746f20746865207a65726f206164647260448201526265737360e81b6064820152608490fd5b60405162461bcd60e51b815260206004820152602560248201527f45524332303a207472616e736665722066726f6d20746865207a65726f206164604482015264647265737360d81b6064820152608490fd5b6001600160a01b03908116918215610de25716918215610d925760207f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925918360005260018252604060002085600052825280604060002055604051908152a3565b60405162461bcd60e51b815260206004820152602260248201527f45524332303a20617070726f766520746f20746865207a65726f206164647265604482015261737360f01b6064820152608490fd5b60405162461bcd60e51b8152602060048201526024808201527f45524332303a20617070726f76652066726f6d20746865207a65726f206164646044820152637265737360e01b6064820152608490fd5b90816020910312610b7057516001600160a01b0381168103610b70579056fea2646970667358221220285c200b3978b10818ff576bb83f2dc4a2a7c98dfb6a36ea01170de792aa652764736f6c63430008140033000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000d3fd4f95820a9aa848ce716d6c200eaefb9a2e4900000000000000000000000000000000000000000000000000000000000000640000000000000000000000000000000000000000000000000000000000000003543131000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000035431310000000000000000000000000000000000000000000000000000000000c001a04e551c75810ffdfe6caff57da9f5a8732449f42f0f4c57f935b05250a76db3b6a046cd47e6d01914270c1ec0d9ac7fae7dfb240ec9a8b6ec7898c4d6aa174388f2";

        let data = hex::decode(raw).unwrap();
        let tx = PooledTransaction::decode_2718(&mut data.as_ref()).unwrap();

        EthPooledTransaction::from_pooled(tx.try_into_recovered().unwrap())
    }

    // https://etherscan.io/tx/0xbcc1267b368fb7ec3b3dbe4ea4adc16159cfba36c425bd2b58880c2c7f4ee557
    fn get_swap_transaction() -> EthPooledTransaction {
        let raw = "0x02f902fe01808405f5e100850236d5cb038302f4d9941111111254eeb25477b68fb85ed929f73a96058288016ed7d0b8a67e78b9028812aa3caf0000000000000000000000003451b6b219478037a1ac572706627fc2bda1e812000000000000000000000000eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee000000000000000000000000cab254f1a32343f11ab41fbde90ecb410cde348a0000000000000000000000003451b6b219478037a1ac572706627fc2bda1e8120000000000000000000000008f1448d143045ae5c30df2fd263becee8e0d30e1000000000000000000000000000000000000000000000000016ed7d0b8a67e7800000000000000000000000000000000000000001bfa2773c00efbe992abd515000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000000000000000000160000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ef0000000000000000000000000000000000000000d100006e00005400004e802026678dcd0000000000000000000000000000000000000000382ffce2287252f930e1c8dc9328dac5bf282ba10000000000000000000000000000000000000000000000000003ab1e3f49584e00206b4be0b94041c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2d0e30db002a000000000000000000000000000000000000000001bfa2773c00efbe992abd515ee63c1e5815628f3bb1f352f86ea173184ffee2e34b8fc2dc8c02aaa39b223fe8d0a0e5c4f27ead9083c756cc21111111254eeb25477b68fb85ed929f73a96058200000000000000000000000000000000009a635db5c001a02df396b33735e66a161f94634d4d06e2d58e748e139c1a05c8ecaf29a7741132a06b31214ffd34846c343c25ab80e228e4d6ba0ef226ee19829bdcafb5889650ec";

        let data = hex::decode(raw).unwrap();
        let tx = PooledTransaction::decode_2718(&mut data.as_ref()).unwrap();

        EthPooledTransaction::from_pooled(tx.try_into_recovered().unwrap())
    }

    fn assert_outcome_tx_type_not_supported(
        outcome: TransactionValidationOutcome<EthPooledTransaction>,
    ) {
        if let TransactionValidationOutcome::Invalid(_, InvalidPoolTransactionError::Other(x)) =
            outcome
        {
            if x.to_string() == *"non-allowed transaction sender and recipient" {
                return;
            }
        }

        panic!("expected NonAllowedSenderAndRecipient")
    }

    #[tokio::test]
    async fn denies_non_allowed_transactions() {
        let provider = MockEthProvider::default();
        let blob_store = InMemoryBlobStore::default();
        let transaction = get_swap_transaction();
        // Deny everything
        let config = IvmConfig::deny_all();
        // Make sure the sender has enough gas
        provider.add_account(
            transaction.sender(),
            ExtendedAccount::new(transaction.nonce(), U256::MAX),
        );

        let validator: IvmTransactionValidator<MockEthProvider, EthPooledTransaction> =
            IvmTransactionValidatorBuilder::new(MAINNET.clone()).build(
                provider,
                blob_store.clone(),
                config,
            );
        // Set timestamp to 1 to avoid genesis edge case
        validator.set_timestamp(1);

        let pool = Pool::new(
            validator.clone(),
            GaslessOrdering::default(),
            blob_store,
            Default::default(),
        );

        let outcome = validator.validate_one(TransactionOrigin::External, transaction.clone());
        // The outcome is invalid
        assert_outcome_tx_type_not_supported(outcome);
        // Its an error when we try and add the transaction
        let pool_err = pool.add_external_transaction(transaction.clone()).await.unwrap_err();
        assert!(pool_err.to_string().contains("non-allowed transaction sender and recipient"));
        // Pool does not persist the transaction
        assert!(pool.get(transaction.hash()).is_none());
    }

    #[tokio::test]
    async fn allows_valid_senders_basic() {
        let provider = MockEthProvider::default();
        let blob_store = InMemoryBlobStore::default();
        let transaction = get_swap_transaction();
        let other_transaction = get_create_transaction();

        // Set the allowed `sender`s
        let mut allow_config = IvmTransactionAllowConfig::deny_all();
        allow_config.add_sender(transaction.sender());
        let mut config = IvmConfig::deny_all();
        config.set_fork(0, allow_config);

        // Make sure the sender has enough gas
        provider.add_account(
            transaction.sender(),
            ExtendedAccount::new(transaction.nonce(), U256::MAX),
        );
        provider.add_account(
            other_transaction.sender(),
            ExtendedAccount::new(transaction.nonce(), U256::MAX),
        );

        let validator: IvmTransactionValidator<MockEthProvider, EthPooledTransaction> =
            IvmTransactionValidatorBuilder::new(MAINNET.clone()).build(
                provider,
                blob_store.clone(),
                config,
            );
        // Set timestamp to 1 to avoid genesis edge case
        validator.set_timestamp(1);

        let pool = Pool::new(
            validator.clone(),
            GaslessOrdering::default(),
            blob_store,
            Default::default(),
        );
        let outcome = validator.validate_one(TransactionOrigin::External, transaction.clone());

        // Validator says transaction is valid
        assert!(outcome.is_valid());

        // Pool persists transaction
        pool.add_external_transaction(transaction.clone()).await.unwrap();
        assert_eq!(pool.get(transaction.hash()).unwrap().hash(), transaction.hash());

        // And we check that it still denies a transaction that's not allowed
        assert_ne!(other_transaction.sender(), transaction.sender());
        let outcome =
            validator.validate_one(TransactionOrigin::External, other_transaction.clone());
        // The outcome is invalid
        assert_outcome_tx_type_not_supported(outcome);
        // Its an error when we try and add the transaction
        let pool_err = pool.add_external_transaction(other_transaction.clone()).await.unwrap_err();
        assert!(pool_err.to_string().contains("non-allowed transaction sender and recipient"));
        // Pool does not persist the transaction
        assert!(pool.get(other_transaction.hash()).is_none());
    }

    #[tokio::test]
    async fn allows_valid_to() {
        let provider = MockEthProvider::default();
        let blob_store = InMemoryBlobStore::default();
        let transaction = get_swap_transaction();
        let other_transaction = get_create_transaction();
        let mut allow_config = IvmTransactionAllowConfig::deny_all();

        // Set the allowed `to` addresses
        allow_config.add_to(transaction.to().unwrap());
        let mut config = IvmConfig::deny_all();
        config.set_fork(0, allow_config);

        // Make sure the sender has enough gas
        provider.add_account(
            transaction.sender(),
            ExtendedAccount::new(transaction.nonce(), U256::MAX),
        );
        provider.add_account(
            other_transaction.sender(),
            ExtendedAccount::new(transaction.nonce(), U256::MAX),
        );

        let validator: IvmTransactionValidator<MockEthProvider, EthPooledTransaction> =
            IvmTransactionValidatorBuilder::new(MAINNET.clone()).build(
                provider,
                blob_store.clone(),
                config,
            );
        // Set timestamp to 1 to avoid genesis edge case
        validator.set_timestamp(1);

        let pool = Pool::new(
            validator.clone(),
            GaslessOrdering::default(),
            blob_store,
            Default::default(),
        );

        // validator says transaction valid
        let outcome = validator.validate_one(TransactionOrigin::External, transaction.clone());
        assert!(outcome.is_valid());

        // pool persists transaction
        pool.add_external_transaction(transaction.clone()).await.unwrap();
        assert_eq!(pool.get(transaction.hash()).unwrap().hash(), transaction.hash());

        // And we check that it still denies a transaction that's not allowed
        assert_ne!(other_transaction.to(), transaction.to());
        let outcome =
            validator.validate_one(TransactionOrigin::External, other_transaction.clone());
        // The outcome is invalid
        assert_outcome_tx_type_not_supported(outcome);
        // Its an error when we try and add the transaction
        let pool_err = pool.add_external_transaction(other_transaction.clone()).await.unwrap_err();
        assert!(pool_err.to_string().contains("non-allowed transaction sender and recipient"));
        // Pool does not persist the transaction
        assert!(pool.get(other_transaction.hash()).is_none());
    }

    #[tokio::test]
    async fn allows_all_when_all_is_set() {
        let provider = MockEthProvider::default();
        let blob_store = InMemoryBlobStore::default();
        let transaction = get_swap_transaction();
        let other_transaction = get_create_transaction();
        let mut allow_config = IvmTransactionAllowConfig::deny_all();

        // Set to allow all addresses
        allow_config.set_all(true);
        let mut config = IvmConfig::deny_all();
        config.set_fork(0, allow_config);

        // Make sure the sender has enough gas
        provider.add_account(
            transaction.sender(),
            ExtendedAccount::new(transaction.nonce(), U256::MAX),
        );
        provider.add_account(
            other_transaction.sender(),
            ExtendedAccount::new(other_transaction.nonce(), U256::MAX),
        );

        let validator: IvmTransactionValidator<MockEthProvider, EthPooledTransaction> =
            IvmTransactionValidatorBuilder::new(MAINNET.clone()).build(
                provider,
                blob_store.clone(),
                config,
            );
        // Set timestamp to 1 to avoid genesis edge case
        validator.set_timestamp(1);

        let pool = Pool::new(
            validator.clone(),
            GaslessOrdering::default(),
            blob_store,
            Default::default(),
        );

        let outcome = validator.validate_one(TransactionOrigin::External, transaction.clone());
        assert!(outcome.is_valid());

        pool.add_external_transaction(transaction.clone()).await.unwrap();
        assert_eq!(pool.get(transaction.hash()).unwrap().hash(), transaction.hash());

        // Also allows the other transaction
        let outcome =
            validator.validate_one(TransactionOrigin::External, other_transaction.clone());
        assert!(outcome.is_valid());

        pool.add_external_transaction(other_transaction.clone()).await.unwrap();
        assert_eq!(pool.get(other_transaction.hash()).unwrap().hash(), other_transaction.hash());
    }

    #[tokio::test]
    async fn allows_valid_to_with_no_balance() {
        let provider = MockEthProvider::default();
        let blob_store = InMemoryBlobStore::default();
        let transaction = get_swap_transaction();
        let other_transaction = get_create_transaction();
        let mut allow_config = IvmTransactionAllowConfig::deny_all();

        // Set the allowed `to` addresses
        allow_config.add_to(transaction.to().unwrap());
        let mut config = IvmConfig::deny_all();
        config.set_fork(0, allow_config);

        // Check that the account balances are non-existent
        assert!(provider.account_balance(&transaction.sender()).unwrap().is_none());
        assert!(provider.account_balance(&other_transaction.sender()).unwrap().is_none());

        let validator: IvmTransactionValidator<MockEthProvider, EthPooledTransaction> =
            IvmTransactionValidatorBuilder::new(MAINNET.clone()).build(
                provider,
                blob_store.clone(),
                config,
            );
        // Set timestamp to 1 to avoid genesis edge case
        validator.set_timestamp(1);

        let pool = Pool::new(
            validator.clone(),
            GaslessOrdering::default(),
            blob_store,
            Default::default(),
        );

        // validator says transaction valid
        let outcome = validator.validate_one(TransactionOrigin::External, transaction.clone());
        assert!(outcome.is_valid());

        // pool persists transaction
        pool.add_external_transaction(transaction.clone()).await.unwrap();
        assert_eq!(pool.get(transaction.hash()).unwrap().hash(), transaction.hash());

        // And we check that it still denies a transaction that's not allowed
        assert_ne!(other_transaction.to(), transaction.to());
        let outcome =
            validator.validate_one(TransactionOrigin::External, other_transaction.clone());
        // The outcome is invalid
        assert_outcome_tx_type_not_supported(outcome);
        // Its an error when we try and add the transaction
        let pool_err = pool.add_external_transaction(other_transaction.clone()).await.unwrap_err();
        assert!(pool_err.to_string().contains("non-allowed transaction sender and recipient"));
        // Pool does not persist the transaction
        assert!(pool.get(other_transaction.hash()).is_none());
    }

    #[tokio::test]
    async fn allows_valid_senders_with_no_balance() {
        let provider = MockEthProvider::default();
        let blob_store = InMemoryBlobStore::default();
        let transaction = get_swap_transaction();
        let other_transaction = get_create_transaction();

        // Set the allowed `sender`s
        let mut allow_config = IvmTransactionAllowConfig::deny_all();
        allow_config.add_sender(transaction.sender());
        let mut config = IvmConfig::deny_all();
        config.set_fork(0, allow_config);

        // Make sure the sender has enough gas
        assert!(provider.account_balance(&transaction.sender()).unwrap().is_none());
        assert!(provider.account_balance(&other_transaction.sender()).unwrap().is_none());

        let validator: IvmTransactionValidator<MockEthProvider, EthPooledTransaction> =
            IvmTransactionValidatorBuilder::new(MAINNET.clone()).build(
                provider,
                blob_store.clone(),
                config,
            );
        // Set timestamp to 1 to avoid genesis edge case
        validator.set_timestamp(1);

        let pool = Pool::new(
            validator.clone(),
            GaslessOrdering::default(),
            blob_store,
            Default::default(),
        );
        let outcome = validator.validate_one(TransactionOrigin::External, transaction.clone());

        // Validator says transaction is valid
        assert!(outcome.is_valid());

        // Pool persists transaction
        pool.add_external_transaction(transaction.clone()).await.unwrap();
        assert_eq!(pool.get(transaction.hash()).unwrap().hash(), transaction.hash());

        // And we check that it still denies a transaction that's not allowed
        assert_ne!(other_transaction.sender(), transaction.sender());
        let outcome =
            validator.validate_one(TransactionOrigin::External, other_transaction.clone());
        // The outcome is invalid
        assert_outcome_tx_type_not_supported(outcome);
        // Its an error when we try and add the transaction
        let pool_err = pool.add_external_transaction(other_transaction.clone()).await.unwrap_err();
        assert!(pool_err.to_string().contains("non-allowed transaction sender and recipient"));
        // Pool does not persist the transaction
        assert!(pool.get(other_transaction.hash()).is_none());
    }

    #[tokio::test]
    async fn allows_valid_senders_with_low_balance() {
        let provider = MockEthProvider::default();
        let blob_store = InMemoryBlobStore::default();
        let transaction = get_swap_transaction();
        let other_transaction = get_create_transaction();

        // Set the allowed `sender`s
        let mut allow_config = IvmTransactionAllowConfig::deny_all();
        allow_config.add_sender(transaction.sender());
        let mut config = IvmConfig::deny_all();
        config.set_fork(0, allow_config);

        // Make sure the sender has some gas, but not enough for the transaction
        provider.add_account(
            transaction.sender(),
            ExtendedAccount::new(transaction.nonce(), U256::from(10)),
        );
        provider.add_account(
            other_transaction.sender(),
            ExtendedAccount::new(other_transaction.nonce(), U256::from(10)),
        );

        let validator: IvmTransactionValidator<MockEthProvider, EthPooledTransaction> =
            IvmTransactionValidatorBuilder::new(MAINNET.clone()).build(
                provider,
                blob_store.clone(),
                config,
            );
        // Set timestamp to 1 to avoid genesis edge case
        validator.set_timestamp(1);

        let pool = Pool::new(
            validator.clone(),
            GaslessOrdering::default(),
            blob_store,
            Default::default(),
        );
        let outcome = validator.validate_one(TransactionOrigin::External, transaction.clone());

        // Validator says transaction is valid
        assert!(outcome.is_valid());

        // Pool persists transaction
        pool.add_external_transaction(transaction.clone()).await.unwrap();
        assert_eq!(pool.get(transaction.hash()).unwrap().hash(), transaction.hash());

        // And we check that it still denies a transaction that's not allowed
        assert_ne!(other_transaction.sender(), transaction.sender());
        let outcome =
            validator.validate_one(TransactionOrigin::External, other_transaction.clone());
        // The outcome is invalid
        assert_outcome_tx_type_not_supported(outcome);
        // Its an error when we try and add the transaction
        let pool_err = pool.add_external_transaction(other_transaction.clone()).await.unwrap_err();
        assert!(pool_err.to_string().contains("non-allowed transaction sender and recipient"));
        // Pool does not persist the transaction
        assert!(pool.get(other_transaction.hash()).is_none());
    }
}

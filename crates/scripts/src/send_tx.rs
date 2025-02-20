//! A script to send some transactions to a local node. The transactions are arbitrary and is mostly
//! useful to sanity check.
//!
//! It covers sending a balance transfer, deploying a contract and calling a contract method.

use alloy::{
    network::{EthereumWallet, TransactionBuilder},
    primitives::U256,
    providers::{ext::TraceApi, Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    sol,
};
use ivm_test_utils::get_signers;

const DEFAULT_DEV_HTTP: &str = "http://127.0.0.1:8545";

// Codegen from embedded Solidity code and precompiled bytecode.
sol! {
    #[allow(missing_docs)]
    // solc v0.8.26; solc Counter.sol --via-ir --optimize --bin
    #[sol(rpc, bytecode="6080806040523460135760df908160198239f35b600080fdfe6080806040526004361015601257600080fd5b60003560e01c9081633fb5c1cb1460925781638381f58a146079575063d09de08a14603c57600080fd5b3460745760003660031901126074576000546000198114605e57600101600055005b634e487b7160e01b600052601160045260246000fd5b600080fd5b3460745760003660031901126074576020906000548152f35b34607457602036600319011260745760043560005500fea2646970667358221220e978270883b7baed10810c4079c941512e93a7ba1cd1108c781d4bc738d9090564736f6c634300081a0033")]
    contract Counter {
        uint256 public number;

        function setNumber(uint256 newNumber) public {
            number = newNumber;
        }

        function increment() public {
            number++;
        }
    }
}

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // Normally the first 20 wallets are funded so we try to get a non-funded one
    let wallets = get_signers(22);
    let bob_wallet = EthereumWallet::from(wallets[21].clone());
    let bob_address = wallets[21].address();
    tracing::info!(?bob_address);

    let alice_address = wallets[20].address();
    tracing::info!(?alice_address);

    let provider = ProviderBuilder::new()
        .wallet(bob_wallet.clone())
        .on_http(DEFAULT_DEV_HTTP.parse().unwrap());

    let tx = TransactionRequest::default()
        .with_to(alice_address)
        .with_from(bob_address)
        .with_value(U256::from(0));

    let tx_receipt = provider.send_transaction(tx).await.unwrap().get_receipt().await.unwrap();
    // This only works if the trace rpc endpoint is enabled
    let tracing_result = provider.trace_transaction(tx_receipt.transaction_hash).await.unwrap();
    tracing::info!(?tracing_result);

    let counter_contract = Counter::deploy(&provider).await.unwrap();

    let call = counter_contract.increment();
    let tx_receipt = call.send().await.unwrap().get_receipt().await.unwrap();
    let tracing_result = provider.trace_transaction(tx_receipt.transaction_hash).await.unwrap();
    tracing::info!(?tracing_result);
}

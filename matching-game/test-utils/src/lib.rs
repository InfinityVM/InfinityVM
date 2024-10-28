//! High level test utilities specifically for the matching game.

use matching_game_core::{api::Request, tick, BorshKeccak256, MatchingGameState};

use alloy::{
    network::EthereumWallet,
    primitives::Address,
    providers::ProviderBuilder,
    signers::{
        k256::ecdsa::SigningKey,
        local::{LocalSigner, PrivateKeySigner},
    },
};
use keccak_hasher::KeccakHasher;
use matching_game_contracts::matching_game_consumer::MatchingGameConsumer;
use memory_db::{HashKey, MemoryDB};
use reference_trie::{ExtensionLayout, RefTrieDBMutBuilder};
use test_utils::{get_signers, AnvilJobManager};
use trie_db::{TrieDBMut, TrieMut};

/// Local Signer
pub type K256LocalSigner = LocalSigner<SigningKey>;

/// Output form [`anvil_with_matching_game_consumer`]
#[derive(Debug)]
pub struct AnvilMatchingGame {
    /// Offchain signer for matching game.
    pub matching_game_signer: PrivateKeySigner,
    /// Address of the matching game consumer contract
    pub matching_game_consumer: Address,
}

/// Deploy `MatchingGameConsumer` to anvil instance.
pub async fn anvil_with_matching_game_consumer(anvil: &AnvilJobManager) -> AnvilMatchingGame {
    let AnvilJobManager { anvil, job_manager, .. } = anvil;
    matching_game_consumer_deploy(anvil.endpoint(), job_manager).await
}

/// Deploy matching game consumer contracts.
pub async fn matching_game_consumer_deploy(
    rpc_url: String,
    job_manager: &Address,
) -> AnvilMatchingGame {
    let signers = get_signers(6);

    let consumer_owner: PrivateKeySigner = signers[4].clone();
    let matching_game_signer: PrivateKeySigner = signers[5].clone();

    let consumer_owner_wallet = EthereumWallet::from(consumer_owner.clone());

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(consumer_owner_wallet)
        .on_http(rpc_url.parse().unwrap());

    let mut memory_db = MemoryDB::<KeccakHasher, HashKey<KeccakHasher>, Vec<u8>>::default();
    let mut initial_root = Default::default();
    let mut merkle_trie = RefTrieDBMutBuilder::new(&mut memory_db, &mut initial_root).build();
    let init_state_hash = *merkle_trie.root();

    // Deploy the matching game consumer
    let matching_game_consumer = *MatchingGameConsumer::deploy(
        provider,
        *job_manager,
        matching_game_signer.address(),
        0,
        init_state_hash.into(),
    )
    .await
    .unwrap()
    .address();

    AnvilMatchingGame { matching_game_signer, matching_game_consumer }
}

/// Returns the next state given a list of transactions.
pub fn next_state<'a>(
    txns: Vec<Request>,
    init_state: MatchingGameState,
    merkle_trie: TrieDBMut<'a, ExtensionLayout>,
) -> (MatchingGameState, TrieDBMut<'a, ExtensionLayout>) {
    let mut next_matching_game_state = init_state;
    let mut next_merkle_trie = merkle_trie;
    // for tx in txns.iter().cloned() {
    //     (_, next_matching_game_state, next_merkle_trie) =
    //         tick(tx, next_matching_game_state, next_merkle_trie);
    // }

    (next_matching_game_state, next_merkle_trie)
}

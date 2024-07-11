//! CLI for zkvm executor gRPC server.

use alloy::{primitives::hex, signers::local::LocalSigner};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
};

use clap::{Parser, Subcommand, ValueEnum};
use k256::ecdsa::SigningKey;

use crate::{service::ZkvmExecutorService, DEV_SECRET};

/// Errors from the executor CLI
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// private key was not valid hex
    #[error("private key was not valid hex")]
    InvalidPrivateKeyHex(#[from] hex::FromHexError),
    /// private key hex was too short
    #[error("private key hex was too short")]
    ShortPrivateKeyHex,
    /// invalid private key
    #[error("invalid private key: {0}")]
    Ecdsa(#[from] k256::ecdsa::Error),
    /// error creating alloy signer
    #[error("error creating signer: {0}")]
    Signer(#[from] alloy::signers::Error),
    /// grpc server failure
    #[error("grpc server failure: {0}")]
    GrpcServer(#[from] tonic::transport::Error),
    /// errors from alloy signer local crate
    #[error(transparent)]
    SignerLocal(#[from] alloy_signer_local::LocalSignerError),
}

type K256LocalSigner = LocalSigner<SigningKey>;

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum Zkvm {
    Risc0,
    Sp1,
}

impl std::fmt::Display for Zkvm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value().expect("no values are skipped").get_name().fmt(f)
    }
}

#[derive(Debug, Clone, Subcommand)]
#[command(version, about, long_about = None)]
enum Operator {
    /// Use a development key
    Dev,
    /// Use an encrypted keystore
    KeyStore(KeyStore),
    /// Pass the hex encoded secret in at the command line
    Secret(Secret),
}

#[derive(Debug, Clone, clap::Args)]
struct KeyStore {
    /// Path to JSON keystore
    #[arg(long, value_name = "FILE")]
    path: PathBuf,
    /// Password for decrypting the JSON keystore
    #[arg(long)]
    password: String,
}

#[derive(Debug, Clone, clap::Args)]
struct Secret {
    secret: String,
}

/// Zkvm execution service.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Opts {
    /// IP address to listen on
    #[arg(long)]
    ip: Ipv4Addr,
    /// Port to listen to listen on
    #[arg(long)]
    port: u16,
    /// Chain ID of where results are expected to get submitted.
    #[arg(long)]
    chain_id: Option<u64>,
    #[command(subcommand)]
    operator_key: Operator,
}

impl Opts {
    fn operator_signer(&self) -> Result<K256LocalSigner, Error> {
        let signer = match &self.operator_key {
            Operator::Dev => K256LocalSigner::from_slice(&DEV_SECRET)?,
            Operator::KeyStore(KeyStore { path, password }) => {
                K256LocalSigner::decrypt_keystore(path, password)?
            }
            Operator::Secret(Secret { secret }) => {
                if secret.as_bytes().len() < 64 {
                    return Err(Error::ShortPrivateKeyHex);
                }
                let hex = if secret[0..2] == *"0x" { &secret[2..] } else { &secret[..] };
                let decoded = hex::decode(hex)?;
                K256LocalSigner::from_slice(&decoded)?
            }
        };

        Ok(signer)
    }
}

/// Command line interface for running the executor gRPC server.
#[derive(Parser, Debug)]
pub struct Cli;

impl Cli {
    /// Run the CLI
    pub async fn run() -> Result<(), Error> {
        let opts = Opts::parse();

        let addr = SocketAddrV4::new(opts.ip, opts.port);
        let signer = opts.operator_signer()?;

        // TODO(zeke): add option for rocksdb
        let db = kvdb_memorydb::create(2);

        let executor_service = ZkvmExecutorService::new(signer, opts.chain_id, db);

        let executor = proto::zkvm_executor_server::ZkvmExecutorServer::new(executor_service);

        // TODO: figure out reflection service protos
        // let reflector = tonic_reflection::server::Builder::configure()
        //     .register_encoded_file_descriptor_set(TODO)
        //     .build()
        //     .expect("failed to start reflector service");

        tonic::transport::Server::builder()
            .add_service(executor)
            // .add_service(reflector)
            .serve(addr.into())
            .await
            .map_err(Into::into)
        // .serve_with_shutdown(addr, async {
        //     // TODO
        // })
    }
}

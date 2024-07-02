//! Better then your server.
use alloy::signers::local::LocalSigner;
use std::{
    net::{Ipv4Addr, SocketAddrV4},
};

use clap::{Parser, ValueEnum};
use k256::ecdsa::SigningKey;
use service::ZkvmExecutorService;

mod service;

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

// #[derive(Parser, Debug)]
// #[derive(Subcommand, Debug, Display)]
// enum Operator {
//     Dev {
//         /// Use the hardcoded account. DO NOT USE IN PRODUCTION
//         dev: bool
//     },
//     /// Encrypted keystore
//     KeyStore {
//         // #[arg(long, value_name = "FILE")]
//         key_store_path: PathBuf,
//         /// Password for decrypting the JSON key store
//         // #[arg(long)]
//         key_store_password: String,
//     }
// }

/// Zkvm execution service.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// IP address to listen on.
    #[arg(long)]
    ip: Ipv4Addr,
    /// Port to listen to listen on.
    #[arg(long)]
    port: u16,
    /// ZKVM variant to run
    #[arg(
        long,
        default_value_t = Zkvm::Risc0,
        default_missing_value = "risc0"
    )]
    zkvm: Zkvm,
    /// Path to encrypted JSON key store

    /// Chain ID of where results are expected to get submitted.
    #[arg(long)]
    chain_id: Option<u64>,
    // #[command(subcommand)]
    // operator: Operator,
}

// impl std::fmt::Display for Operator {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         self.to_possible_value()
//             .expect("no values are skipped")
//             .get_name()
//             .fmt(f)
//     }
// }

impl Args {
    fn operator_wallet(&self) -> K256LocalSigner {
        // let mut  K256LocalSigner::from
        // if let Some(chain_id) = self.chain_id {

        // } else {

        // }
        unimplemented!()
    }
}

/// Command line interface for running the executor gRPC server.
#[derive(Parser, Debug)]
pub struct Cli;

impl Cli {
    /// Run the CLI
    pub async fn run() -> Result<(), tonic::transport::Error> {
        let opts = Args::parse();

        let addr = SocketAddrV4::new(opts.ip, opts.port);
        let wallet = opts.operator_wallet();

        let executor_service = match opts.zkvm {
            Zkvm::Risc0 => ZkvmExecutorService::<zkvm::Risc0, _>::new(wallet, opts.chain_id),
            Zkvm::Sp1 => unimplemented!(),
        };

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
        // .serve_with_shutdown(addr, async {
        //     // TODO
        // })
    }
}

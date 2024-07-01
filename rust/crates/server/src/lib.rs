//! Better then your server.
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddrV4},
    path::PathBuf,
};
use alloy::signers::local::LocalSigner;

use alloy::network::EthereumWallet;
use clap::{Parser, ValueEnum};
use service::ZkvmExecutorService;

mod service;

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum Zkvm {
    Risc0,
    Sp1,
}

impl std::fmt::Display for Zkvm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

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
    #[arg(long, value_name = "FILE")]
    operator_private_key: PathBuf,
}

impl Args {
    fn operator_wallet(&self) -> EthereumWallet {
        unimplemented!()
    }
}

pub struct Cli;

impl Cli {
    pub async fn run() -> Result<(), tonic::transport::Error> {
        let opts = Args::parse();

        let addr = SocketAddrV4::new(opts.ip, opts.port);
        let wallet = opts.operator_wallet();

        let executor_service = match opts.zkvm {
            Zkvm::Risc0 => ZkvmExecutorService::<zkvm::Risc0, LocalSigner>::new(wallet),
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

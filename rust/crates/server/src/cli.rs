//! CLI for zkvm executor gRPC server.

use alloy::primitives::Address;
use proto::service_client::ServiceClient;

use crate::service::Server;
use clap::{Parser, ValueEnum};

use hyper::{Body, Request, Response, Server as HttpServer, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::net::{SocketAddr, SocketAddrV4};
use tonic::transport::Channel;
use tonic::Request as TonicRequest;
use proto::{
    GetResultRequest, SubmitJobRequest, SubmitProgramRequest,
};

const ENV_RELAYER_PRIV_KEY: &str = "RELAYER_PRIVATE_KEY";

/// Errors from the gRPC Server CLI
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// private key was not valid hex
    #[error("Environment variable {} must be set", ENV_RELAYER_PRIV_KEY)]
    RelayerPrivKeyNotSet,
    /// invalid gRPC address
    #[error("Invalid gRPC address")]
    InvalidGrpcAddress,
    /// grpc server failure
    #[error("grpc server failure: {0}")]
    GrpcServer(#[from] tonic::transport::Error),
}

#[derive(ValueEnum, Debug, Clone)]
enum LoggingFormat {
    Json,
    Text,
}

/// gRPC service.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Opts {
    /// Logging level
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Logging format
    #[arg(long, default_value = "text")]
    log_format: LoggingFormat,

    /// gRPC server address
    #[arg(long, default_value = "127.0.0.1:50051")]
    grpc_address: String,

    /// gRPC gateway server address
    #[arg(long, default_value = "127.0.0.1:8080")]
    grpc_gateway_address: String,

    /// ZK shim address
    #[arg(long, required = true)]
    zk_shim_address: String,

    /// `JobManager` contract address
    #[arg(long, required = true)]
    job_manager_address: Address,

    /// Ethereum RPC address
    #[arg(long, required = true)]
    eth_rpc_address: String,
}

/// Command line interface for running the gRPC server.
#[derive(Parser, Debug)]
pub struct Cli;

impl Cli {
    /// Run the CLI
    pub async fn run() -> Result<(), Error> {
        let opts = Opts::parse();

        // TODO (Maanav): add logging

        let _relayer_private_key =
            std::env::var(ENV_RELAYER_PRIV_KEY).map_err(|_| Error::RelayerPrivKeyNotSet)?;

        // Parse the gRPC address
        let grpc_addr: SocketAddrV4 =
            opts.grpc_address.parse().map_err(|_| Error::InvalidGrpcAddress)?;

        let reflector = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
            .build()
            .expect("failed to build gRPC reflection service");

        println!("Starting gRPC server at: {}", grpc_addr);
        let grpc_server = tonic::transport::Server::builder()
            .add_service(proto::service_server::ServiceServer::new(Server::new()))
            .add_service(reflector)
            .serve(grpc_addr.into());

        // TODO: add HTTP gateway for gRPC server
        let grpc_gateway_addr: SocketAddr = opts.grpc_gateway_address.parse().unwrap();
        let grpc_addr_str = format!("http://{}", grpc_addr);

        let make_svc = make_service_fn(move |_conn| {
            let grpc_addr_str = grpc_addr_str.clone();
            async {
                Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                    let grpc_addr_str = grpc_addr_str.clone();
                    async move {
                        match (req.method(), req.uri().path()) {
                            (&Method::POST, "/submit_job") => {
                                let client = ServiceClient::connect(grpc_addr_str.clone()).await.unwrap();
                                Self::handle_submit_job(client, req).await
                            },
                            (&Method::POST, "/get_result") => {
                                let client = ServiceClient::connect(grpc_addr_str.clone()).await.unwrap();
                                Self::handle_get_result(client, req).await
                            },
                            (&Method::POST, "/submit_program") => {
                                let client = ServiceClient::connect(grpc_addr_str.clone()).await.unwrap();
                                Self::handle_submit_program(client, req).await
                            },
                            _ => Ok::<_, Infallible>(Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(Body::from("Not Found"))
                                .unwrap()),
                        }
                    }
                }))
            }
        });

        let http_server = HttpServer::bind(&grpc_gateway_addr).serve(make_svc);

        println!("Starting HTTP server at: {}", grpc_gateway_addr);

        tokio::select! {
            res = grpc_server => {
                if let Err(e) = res {
                    eprintln!("gRPC server error: {}", e);
                }
            }
            res = http_server => {
                if let Err(e) = res {
                    eprintln!("HTTP server error: {}", e);
                }
            }
        }    

        Ok(())
    }

    async fn handle_submit_job(
        mut client: ServiceClient<Channel>,
        req: Request<Body>
    ) -> Result<Response<Body>, Infallible> {
        let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
        let submit_job_request: SubmitJobRequest = serde_json::from_slice(&whole_body).unwrap();
        let response = client.submit_job(TonicRequest::new(submit_job_request)).await.unwrap();
        let response_body = serde_json::to_string(&response.into_inner()).unwrap();
        Ok(Response::new(Body::from(response_body)))
    }

    async fn handle_get_result(
        mut client: ServiceClient<Channel>,
        req: Request<Body>
    ) -> Result<Response<Body>, Infallible> {
        let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
        let get_result_request: GetResultRequest = serde_json::from_slice(&whole_body).unwrap();
        let response = client.get_result(TonicRequest::new(get_result_request)).await.unwrap();
        let response_body = serde_json::to_string(&response.into_inner()).unwrap();
        Ok(Response::new(Body::from(response_body)))
    }
    
    async fn handle_submit_program(
        mut client: ServiceClient<Channel>,
        req: Request<Body>
    ) -> Result<Response<Body>, Infallible> {
        let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
        let submit_program_request: SubmitProgramRequest = serde_json::from_slice(&whole_body).unwrap();
        let response = client.submit_program(TonicRequest::new(submit_program_request)).await.unwrap();
        let response_body = serde_json::to_string(&response.into_inner()).unwrap();
        Ok(Response::new(Body::from(response_body)))
    }
}

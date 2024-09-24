//! REST gRPC gateway is a reverse proxy that exposes an HTTP interface to the coprocessor-node gRPC
//! routes.

use std::net::SocketAddr;

use proto::{
    coprocessor_node_client::CoprocessorNodeClient, GetResultRequest, GetResultResponse,
    SubmitJobRequest, SubmitJobResponse, SubmitProgramRequest, SubmitProgramResponse,
};
use tokio::{
    net::TcpStream,
    time::{sleep, Duration},
};
use tonic::transport::Channel;

use axum::{
    extract::{DefaultBodyLimit, State},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};

/// Errors for the REST gRPC gateway.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Failure to connect to gRPC server
    #[error("failed to connect to grpc server: {0}")]
    ConnectionFailure(String),
    /// Network IO error
    #[error(transparent)]
    StdIO(#[from] std::io::Error),
    /// invalid gRPC address
    #[error("invalid gRPC address: {0}")]
    InvalidGrpcAddress(#[from] std::net::AddrParseError),
}

const SUBMIT_JOB: &str = "submit_job";
const GET_RESULT: &str = "get_result";
const SUBMIT_PROGRAM: &str = "submit_program";

/// Error response from the gateway
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ErrorResponse {
    /// Error message
    pub message: String,
    /// gRPC status code
    pub grpc_code: i32,
}

impl From<tonic::Status> for ErrorResponse {
    fn from(s: tonic::Status) -> Self {
        Self { message: s.message().to_owned(), grpc_code: s.code() as i32 }
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        use axum::http::StatusCode;
        use tonic::Code;

        // https://github.com/grpc/grpc/blob/master/doc/http-grpc-status-mapping.md
        let grpc_code = Code::from_i32(self.grpc_code);
        let http = match grpc_code {
            Code::Internal => StatusCode::BAD_REQUEST,
            Code::Unauthenticated => StatusCode::UNAUTHORIZED,
            Code::PermissionDenied => StatusCode::FORBIDDEN,
            Code::Unimplemented => StatusCode::NOT_FOUND,
            Code::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Code::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::CONFLICT,
        };

        (http, Json(self)).into_response()
    }
}

/// REST Gateway proxy
#[derive(Debug)]
pub struct HttpGrpcGateway {
    /// gRPC address to proxy requests to.
    grpc_addr: String,
    /// Address to listen for HTTP requests on.
    listen_addr: SocketAddr,
}

type Client = CoprocessorNodeClient<Channel>;

impl HttpGrpcGateway {
    /// Create the gateway struct. Does not perform any network IO.
    pub const fn new(grpc_addr: String, listen_addr: SocketAddr) -> Self {
        Self { grpc_addr, listen_addr }
    }

    /// Run the HTTP gateway.
    pub async fn serve(self) -> Result<(), Error> {
        let grpc_client = self.connect_with_retry().await?;

        let app = Router::new()
            .route(&self.path(SUBMIT_JOB), post(Self::submit_job))
            .route(&self.path(GET_RESULT), post(Self::get_result))
            .route(&self.path(SUBMIT_PROGRAM), post(Self::submit_program))
            // make sure we can accept request bodies larger then 2mb
            .layer(DefaultBodyLimit::disable())
            .with_state(grpc_client);

        let listener = tokio::net::TcpListener::bind(self.listen_addr).await?;

        tracing::info!("REST gRPC Gateway listening on {}", listener.local_addr()?);

        axum::serve(listener, app).await?;

        Ok(())
    }

    fn path(&self, resource: &str) -> String {
        format!("/v1/coprocessor_node/{resource}")
    }

    async fn submit_job(
        State(mut client): State<Client>,
        Json(body): Json<SubmitJobRequest>,
    ) -> Result<Json<SubmitJobResponse>, ErrorResponse> {
        let tonic_request = tonic::Request::new(body);
        let response = client.submit_job(tonic_request).await?.into_inner();

        Ok(Json(response))
    }

    async fn get_result(
        State(mut client): State<Client>,
        Json(body): Json<GetResultRequest>,
    ) -> Result<Json<GetResultResponse>, ErrorResponse> {
        let tonic_request = tonic::Request::new(body);
        let response = client.get_result(tonic_request).await?.into_inner();

        Ok(Json(response))
    }

    async fn submit_program(
        State(mut client): State<Client>,
        Json(body): Json<SubmitProgramRequest>,
    ) -> Result<Json<SubmitProgramResponse>, ErrorResponse> {
        let tonic_request = tonic::Request::new(body);
        let response = client.submit_program(tonic_request).await?.into_inner();

        Ok(Json(response))
    }

    async fn connect_with_retry(&self) -> Result<Client, Error> {
        const MAX_RETRIES: u32 = 5;
        const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);

        let mut retry_delay = INITIAL_RETRY_DELAY;
        let mut attempts = 0;

        while attempts < MAX_RETRIES {
            match CoprocessorNodeClient::<Channel>::connect(self.grpc_addr.clone()).await {
                Ok(client) => return Ok(client),
                Err(e) => {
                    attempts += 1;
                    if attempts >= MAX_RETRIES {
                        return Err(Error::ConnectionFailure(e.to_string()));
                    }
                    tracing::warn!(
                        "Failed to connect to gRPC server: {}. Retrying ({}/{})...",
                        e,
                        attempts,
                        MAX_RETRIES
                    );
                    sleep(retry_delay).await;
                    retry_delay *= 2;
                    if retry_delay > Duration::from_secs(60) {
                        retry_delay = Duration::from_secs(60); // Cap at 60 seconds
                    }
                }
            }
        }
        Err(Error::ConnectionFailure("Max retries reached".to_string()))
    }
}

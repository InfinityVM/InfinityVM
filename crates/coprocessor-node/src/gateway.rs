//! REST gRPC gateway is a reverse proxy that exposes an HTTP interface to the coprocessor-node gRPC
//! routes.

use std::{net::SocketAddr, time::Duration};

use proto::{
    coprocessor_node_client::CoprocessorNodeClient, GetResultRequest, GetResultResponse,
    SubmitJobRequest, SubmitJobResponse, SubmitProgramRequest, SubmitProgramResponse,
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

const CONNECT_RETRIES: u64 = 12;
const CONNECT_DELAY_MS: u64 = 250;

type Client = CoprocessorNodeClient<Channel>;

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

impl HttpGrpcGateway {
    /// Create the gateway struct. Does not perform any network IO.
    pub const fn new(grpc_addr: String, listen_addr: SocketAddr) -> Self {
        Self { grpc_addr, listen_addr }
    }

    /// Run the HTTP gateway.
    pub async fn serve(self) -> Result<(), Error> {
        let client_coproc_grpc = format!("http://{}", self.grpc_addr);

        tracing::info!("REST gRPC Gateway attempting to connect to gRPC service at {}", client_coproc_grpc);
        let mut connect_retries = 0;
        let grpc_client = loop {
            match Client::connect(client_coproc_grpc.clone()).await {
                Ok(client) => break client,
                Err(e) => {
                    if connect_retries > CONNECT_RETRIES {
                        return Err(Error::ConnectionFailure(e.to_string()));
                    } else {
                        connect_retries += 1;
                        let sleep_ms = CONNECT_DELAY_MS * connect_retries;
                        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                    }
                }
            }
        };

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
}

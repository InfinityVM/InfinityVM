//! Implementation of the gateway itself.

use std::net::SocketAddr;

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

use crate::Error;

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

    /// Run the the HTTP gateway.
    pub async fn serve(self) -> Result<(), Error> {
        let grpc_client = CoprocessorNodeClient::<Channel>::connect(self.grpc_addr.clone())
            .await
            .map_err(|e| Error::ConnectionFailure(e.to_string()))?;

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

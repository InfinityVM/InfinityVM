use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server as HttpServer, StatusCode};
use proto::service_client::ServiceClient;
use proto::{GetResultRequest, SubmitJobRequest, SubmitProgramRequest};
use serde_json;
use std::convert::Infallible;
use std::net::SocketAddr;
use tonic::transport::Channel;
use tonic::Request as TonicRequest;

/// Starts HTTP gateway for gRPC server.
pub async fn run_http_server(
    grpc_addr: String,
    grpc_gateway_addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let make_svc = make_service_fn(move |_conn| {
        let grpc_addr = grpc_addr.clone();
        async {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let grpc_addr = grpc_addr.clone();
                async move {
                    match (req.method(), req.uri().path()) {
                        (&Method::POST, "/submit_job") => {
                            let client = ServiceClient::connect(grpc_addr.clone()).await.unwrap();
                            handle_submit_job(client, req).await
                        }
                        (&Method::POST, "/get_result") => {
                            let client = ServiceClient::connect(grpc_addr.clone()).await.unwrap();
                            handle_get_result(client, req).await
                        }
                        (&Method::POST, "/submit_program") => {
                            let client = ServiceClient::connect(grpc_addr.clone()).await.unwrap();
                            handle_submit_program(client, req).await
                        }
                        _ => Ok::<_, Infallible>(
                            Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(Body::from("Not Found"))
                                .unwrap(),
                        ),
                    }
                }
            }))
        }
    });

    let http_server = HttpServer::bind(&grpc_gateway_addr).serve(make_svc);
    http_server.await?;

    Ok(())
}

async fn handle_submit_job(
    mut client: ServiceClient<Channel>,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
    let submit_job_request: SubmitJobRequest = serde_json::from_slice(&whole_body).unwrap();
    let response = client.submit_job(TonicRequest::new(submit_job_request)).await.unwrap();
    let response_body = serde_json::to_string(&response.into_inner()).unwrap();
    Ok(Response::new(Body::from(response_body)))
}

async fn handle_get_result(
    mut client: ServiceClient<Channel>,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
    let get_result_request: GetResultRequest = serde_json::from_slice(&whole_body).unwrap();
    let response = client.get_result(TonicRequest::new(get_result_request)).await.unwrap();
    let response_body = serde_json::to_string(&response.into_inner()).unwrap();
    Ok(Response::new(Body::from(response_body)))
}

async fn handle_submit_program(
    mut client: ServiceClient<Channel>,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
    let submit_program_request: SubmitProgramRequest = serde_json::from_slice(&whole_body).unwrap();
    let response = client.submit_program(TonicRequest::new(submit_program_request)).await.unwrap();
    let response_body = serde_json::to_string(&response.into_inner()).unwrap();
    Ok(Response::new(Body::from(response_body)))
}

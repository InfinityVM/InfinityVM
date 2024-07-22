use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use proto::{
    service_client::ServiceClient, GetResultRequest, SubmitJobRequest, SubmitProgramRequest,
};
use serde_json;
use std::{convert::Infallible, net::SocketAddr};
use tonic::{transport::Channel, Request as TonicRequest};

/// Starts gRPC gateway.
pub async fn run_grpc_gateway(
    grpc_gateway_addr: SocketAddr,
    service_client: ServiceClient<Channel>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // let client
    let make_svc = make_service_fn(move |_conn| {
        let service_client1 = service_client.clone();
        async {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let service_client2 = service_client1.clone();
                async move {
                    match (req.method(), req.uri().path()) {
                        (&Method::POST, "/submit_job") => {
                            handle_submit_job(service_client2.clone(), req).await
                        }
                        (&Method::POST, "/get_result") => {
                            handle_get_result(service_client2, req).await
                        }
                        (&Method::POST, "/submit_program") => {
                            handle_submit_program(service_client2, req).await
                        }
                        _ => Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Body::from("Not Found"))
                            .unwrap()),
                    }
                }
            }))
        }
    });

    let grpc_gateway = Server::bind(&grpc_gateway_addr).serve(make_svc);
    grpc_gateway.await?;

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

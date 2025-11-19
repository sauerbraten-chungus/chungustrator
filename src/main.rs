use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use dotenv::dotenv;
use service::chungustrator_rpc::chungustrator_server::ChungustratorServer;
use tokio::sync::{Mutex, mpsc};
use tonic::transport::Server;
use tracing::error;

use chungustrator_enet::chungus_service_client::ChungusServiceClient;

// mod handler;
mod orchestrator;
mod service;

pub mod chungustrator_enet {
    tonic::include_proto!("chungustrator_enet");
}

#[derive(Clone)]
struct AppState {
    tx: mpsc::UnboundedSender<orchestrator::OrchestratorMessage>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_thread_ids(true)
        .init();

    let chungus_stub = ChungusServiceClient::connect("http://127.0.0.1:50051").await?;

    let (tx, rx) = mpsc::unbounded_channel();
    if let Err(e) = orchestrator::Chungustrator::new(rx, chungus_stub).await {
        error!("Error creating chungustrator: {}", e);
    }

    let chungustrator_grpc_service = service::ChungustratorService { tx };

    let svc = ChungustratorServer::new(chungustrator_grpc_service);

    Server::builder()
        .add_service(svc)
        .serve("0.0.0.0:7000".parse().unwrap())
        .await?;

    Ok(())
}

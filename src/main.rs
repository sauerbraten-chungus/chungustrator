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

    // IMPORTANT: Comments for sanity, god my naming sucked ass, fuck.

    // Connect Chungustrator (gRPC Stub) -> Chungusway (gRPC Service)
    // Bidirectional Streaming
    let chungusway_url =
        std::env::var("CHUNGUSWAY_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
    let chungus_stub = loop {
        match ChungusServiceClient::connect(chungusway_url.clone()).await {
            Ok(client) => break client,
            Err(e) => {
                error!("chungusway not reachable at {chungusway_url}: {e}; retrying in 2s");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    };

    // Channel exclusively for Chungustrator (rx) <- ChungustratorService (tx, gRPC Service) <- Matchmaker (gRPC Stub)
    // Proxy to Action
    let (tx, rx) = mpsc::unbounded_channel();
    if let Err(e) = orchestrator::Chungustrator::new(rx, chungus_stub).await {
        error!("Error creating chungustrator: {}", e);
    }

    // Host Chungustrator (gRPC Service) <- Connection  Matchmaker (gRPC Stub)
    // Request/Response
    let chungustrator_grpc_service = service::ChungustratorService { tx };
    let svc = ChungustratorServer::new(chungustrator_grpc_service);
    Server::builder()
        .add_service(svc)
        .serve(
            format!(
                "0.0.0.0:{}",
                std::env::var("CHUNGUSTRATOR_PORT").unwrap_or_else(|_| "7100".to_string())
            )
            .parse()
            .unwrap(),
        )
        .await?;

    Ok(())
}

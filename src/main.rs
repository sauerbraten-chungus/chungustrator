use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use dotenv::dotenv;
use tokio::sync::{Mutex, mpsc};
use tracing::error;

use chungustrator_enet::auth_code_service_client::AuthCodeServiceClient;

mod handler;
mod orchestrator;

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

    let auth_stub = AuthCodeServiceClient::connect("http://127.0.0.1:50051").await?;

    let (tx, rx) = mpsc::unbounded_channel();
    if let Err(e) = orchestrator::Chungustrator::new(rx, auth_stub).await {
        error!("Error creating chungustrator: {}", e);
    }

    let state = AppState { tx };

    let app = Router::new()
        .route("/health", get(|| async { "Hello World!" }))
        .route("/create", post(handler::create_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

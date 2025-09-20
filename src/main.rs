use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use dotenv::dotenv;
use tokio::sync::{Mutex, mpsc};
use tracing::error;

mod handler;
mod orchestrator;

#[derive(Clone)]
struct AppState {
    tx: mpsc::UnboundedSender<orchestrator::OrchestratorMessage>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_thread_ids(true)
        .init();

    let (tx, rx) = mpsc::unbounded_channel();
    if let Err(e) = orchestrator::Chungustrator::new(rx).await {
        error!("Error creating chungustrator: {}", e);
    }

    let state = AppState { tx };

    let app = Router::new()
        .route("/health", get(|| async { "Hello World!" }))
        .route("/create", post(handler::create_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

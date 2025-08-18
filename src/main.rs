use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

mod handler;
mod orchestrator;

#[derive(Clone)]
struct AppState {
    chungustrator: Arc<orchestrator::Chungustrator>,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        chungustrator: Arc::new(
            orchestrator::Chungustrator::new().expect("Maybe failed because docker xD"),
        ),
    };

    let app = Router::new()
        .route("/health", get(|| async { "Hello World!" }))
        .route("/create", post(handler::create_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

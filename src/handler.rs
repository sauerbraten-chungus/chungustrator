use crate::orchestrator::{OrchestratorMessage, OrchestratorResponse};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use bollard::models::ContainerCreateResponse;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::error;

use crate::AppState;

#[derive(Serialize)]
struct CreateSuccessResponse {
    id: String,
    wan_ip: String,
    lan_ip: String,
    port: u16,
}

#[derive(Serialize)]
struct CreateFailedResponse {
    error: String,
}

fn create_response_channel() -> (
    mpsc::UnboundedSender<OrchestratorResponse>,
    mpsc::UnboundedReceiver<OrchestratorResponse>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    (tx, rx)
}

pub async fn create_handler(State(state): State<AppState>) -> impl IntoResponse {
    let (response_tx, mut response_rx) = create_response_channel();
    let message_tx = state.tx.clone();

    if let Err(e) = message_tx.send(OrchestratorMessage::CreateContainer { response_tx }) {
        error!("Error sending message to chungustrator: {}", e);
    };

    let http_response = match response_rx.recv().await {
        Some(OrchestratorResponse::ContainerCreationSuccess {
            id,
            wan_ip,
            lan_ip,
            port,
        }) => (
            StatusCode::OK,
            Json(CreateSuccessResponse {
                id,
                wan_ip,
                lan_ip,
                port,
            }),
        )
            .into_response(),
        Some(OrchestratorResponse::ContainerCreationError { error }) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateFailedResponse { error }),
        )
            .into_response(),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateFailedResponse {
                error: "No OrchestratorResponse".to_string(),
            }),
        )
            .into_response(),
    };

    http_response
}

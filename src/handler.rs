use crate::orchestrator::ContainerCreationResult;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;

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

pub async fn create_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut chungustrator = state.chungustrator.lock().await;
    let create_result = chungustrator.create_container().await;

    match create_result {
        Ok(result) => {
            // stuff
            println!("{}", result.id);
            (
                StatusCode::OK,
                Json(CreateSuccessResponse {
                    id: result.id,
                    wan_ip: result.wan_ip,
                    lan_ip: result.lan_ip,
                    port: result.port,
                }),
            )
                .into_response()
        }
        Err(e) => {
            // stuff
            println!("{}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateFailedResponse {
                    error: e.to_string(),
                }),
            )
                .into_response()
        }
    }
}

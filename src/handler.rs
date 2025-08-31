use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
struct CreateSuccessResponse {
    id: String,
}

#[derive(Serialize)]
struct CreateFailedResponse {
    error: String,
}

pub async fn create_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.chungustrator.create_container().await {
        Ok(id) => {
            // stuff
            println!("{}", id);
            (StatusCode::OK, Json(CreateSuccessResponse { id })).into_response()
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

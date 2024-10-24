use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use db_access::auth::add_api_key;

#[derive(Deserialize)]
pub struct ApiKeyRequest {
    name: String,
}

#[derive(Serialize)]
pub struct ApiKeyResponse {
    api_key: String,
}

pub async fn create_api_key(
    State(state): State<AppState>,
    Json(payload): Json<ApiKeyRequest>,
) -> Result<Json<ApiKeyResponse>, StatusCode> {
    let api_key = Uuid::new_v4().to_string();

    if let Err(e) = add_api_key(&state.db.pool, api_key.clone(), payload.name).await {
        tracing::error!("Failed to store API key: {:?}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(ApiKeyResponse { api_key }))
}

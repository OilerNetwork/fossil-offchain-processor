use axum::{extract::State, response::IntoResponse, Json};

use crate::{AppState, StorageRequest};

pub async fn get_storage_value(
    State(app_state): State<AppState>, //
    Json(input): Json<StorageRequest>,
) -> impl IntoResponse {
    let account_address = input.account_address;
    let storage_key = input.storage_key;

    let cache_key = format!("{}:{}", account_address, storage_key);

    let mut cache = app_state.storage_cache.lock().unwrap();

    if let Some(cached_value) = cache.get(&cache_key) {
        return Json(serde_json::json!({
            "status": "cached",
            "value": cached_value,
        }))
        .into_response();
    }

    drop(cache); // unlock the mutex

    Json(serde_json::json!({
        "status": "not_found_in_cache",
        "message": "Proceed to fetch from Ethereum Data Dispatcher"
    }))
    .into_response()
}

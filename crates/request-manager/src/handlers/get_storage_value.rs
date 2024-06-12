use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use reqwest::Client;
use serde::Deserialize;
use std::env;

use crate::state::AppState;
use crate::utils::response_with_status;
use proof_generator::model::eth_rpc::{EthRpcBody, EthRpcBodyParams};

#[derive(Deserialize)]
pub struct StorageRequest {
    pub account_address: String,
    pub storage_key: String,
}

pub async fn get_storage_value(
    State(app_state): State<AppState>,
    Json(input): Json<StorageRequest>,
) -> impl IntoResponse {
    let account_address = input.account_address.clone();
    let storage_key = input.storage_key.clone();
    let cache_key = format!("{}:{}", account_address, storage_key);

    if let Some(cached_value) = app_state.storage_cache.lock().await.get(&cache_key) {
        return Json(serde_json::json!({
            "status": "cached",
            "value": cached_value,
        }))
        .into_response();
    }

    let block_hash = get_block_hash_from_dispatcher(&app_state.client, &account_address).await;

    match block_hash {
        Ok(block_hash) => {
            let eth_rpc_body = EthRpcBody {
                jsonrpc: "2.0".to_string(),
                method: "eth_getStorageAt".to_string(),
                params: vec![
                    EthRpcBodyParams::AccountAddress(account_address.clone()),
                    EthRpcBodyParams::StorageKeys(vec![storage_key.clone()]),
                    EthRpcBodyParams::BlockIdentifier(block_hash),
                ],
                id: "1".to_string(),
            };

            // let url = env::var("ETH_RPC").expect("ETH_RPC must be set");
            let url = env::var("LOCAL_ETH_RPC_URL").expect("ETH_RPC must be set");

            let response = app_state
                .client
                .post(&url)
                .json(&eth_rpc_body)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .send()
                .await;

            match response {
                Ok(res) => match res.json::<serde_json::Value>().await {
                    Ok(json) => {
                        println!("response: {:#?}", json);

                        if let Some(value) = json.get("result").and_then(|v| v.as_str()) {
                            app_state
                                .storage_cache
                                .lock()
                                .await
                                .insert(cache_key, value.to_string());
                            Json(serde_json::json!({
                                "status": "fetched",
                                "value": value,
                            }))
                            .into_response()
                        } else {
                            response_with_status(StatusCode::BAD_REQUEST, "Invalid response format")
                        }
                    }
                    Err(err) => {
                        eprintln!("Failed to parse JSON: {:?}", err);
                        response_with_status(StatusCode::BAD_REQUEST, "Failed to parse JSON")
                    }
                },
                Err(err) => {
                    eprintln!("Request failed: {:?}", err);
                    response_with_status(StatusCode::INTERNAL_SERVER_ERROR, "Request failed")
                }
            }
        }
        Err(err) => {
            eprintln!("Request failed: {:?}", err);
            response_with_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get block hash",
            )
        }
    }
}

async fn get_block_hash_from_dispatcher(
    client: &Client,
    account_address: &str,
) -> Result<String, ()> {
    // TODO: add connection with Ethereum Data Dispatcher
    Ok("wip".to_string())
}

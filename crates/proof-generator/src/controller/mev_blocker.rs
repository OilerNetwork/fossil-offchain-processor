use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use dotenv::dotenv;
use reqwest::Client;
use serde_json;

use crate::model::eth_rpc::{EthRpcGetProofBody, EthRpcGetProofBodyParams, Input};
use crate::service::calculate_proof::calculate_proof;

pub async fn call_mev_blocker_api(
    State(client): State<Client>,
    Json(input): Json<Input>,
) -> impl IntoResponse {
    dotenv().ok();
    let account_address = input.account_address;
    let storage_keys = input.storage_keys;

    let body = EthRpcGetProofBody {
        jsonrpc: "2.0".to_string(),
        method: "eth_getProof".to_string(),
        params: vec![
            EthRpcGetProofBodyParams::AccountAddress(account_address),
            EthRpcGetProofBodyParams::StorageKeys(storage_keys.clone()),
            EthRpcGetProofBodyParams::BlockIdentifier("latest".to_string()),
        ],
        id: "1".to_string(),
    };

    let url = dotenv::var("ETH_RPC").expect("ETH_RPC must be set");

    let reqwest_response = client
        .post(&url)
        .json(&body)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .send()
        .await;

    match reqwest_response {
        Ok(res) => match res.json::<serde_json::Value>().await {
            Ok(json) => handle_json_response(json, storage_keys),
            Err(_) => response_with_status(StatusCode::BAD_REQUEST, "JSON was not well-formatted"),
        },
        Err(_) => response_with_status(StatusCode::INTERNAL_SERVER_ERROR, "Request failed"),
    }
}

fn handle_json_response(json: serde_json::Value, storage_keys: Vec<String>) -> Response {
    match json {
        serde_json::Value::Object(map) => {
            if let Some(trie_proofs) = map.get("result") {
                if trie_proofs.get("storageProof").is_none()
                    || trie_proofs["storageProof"].as_array().unwrap().is_empty()
                {
                    return response_with_status(StatusCode::BAD_REQUEST, "storageProof is empty");
                }

                match calculate_proof(trie_proofs, &storage_keys) {
                    Ok(response_proof) => (StatusCode::OK, Json(response_proof)).into_response(),
                    Err(err) => response_with_status(StatusCode::BAD_REQUEST, &err.to_string()),
                }
            } else if let Some(error) = map.get("error") {
                response_with_status(
                    StatusCode::BAD_REQUEST,
                    error.get("message").unwrap().as_str().unwrap(),
                )
            } else {
                response_with_status(
                    StatusCode::BAD_REQUEST,
                    "Invalid response from eth_getProof API",
                )
            }
        }
        _ => response_with_status(StatusCode::BAD_REQUEST, "JSON was not well-formatted"),
    }
}

fn response_with_status(status: StatusCode, message: &str) -> Response {
    (status, Json(message)).into_response()
}

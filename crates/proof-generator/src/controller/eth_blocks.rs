use std::env;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use reqwest::Client;

use crate::model::eth_rpc::{
    BlockNumber, EthRpcGetBlockByNumberBody, EthRpcGetBlockByNumberBodyParams,
};

pub async fn call_eth_blocks_api(
    State(client): State<Client>,
    Json(input): Json<BlockNumber>,
) -> impl IntoResponse {
    let url = env::var("ETH_RPC").expect("ETH_RPC must be set");

    let body = EthRpcGetBlockByNumberBody {
        jsonrpc: "2.0".to_string(),
        method: "eth_getBlockByNumber".to_string(),
        params: vec![
            EthRpcGetBlockByNumberBodyParams::BlockIdentifier(input.block_number.hex.clone()),
            EthRpcGetBlockByNumberBodyParams::IncludeTransactions(false),
        ],
        id: "1".to_string(),
    };

    let reqwest_response = client
        .post(&url)
        .json(&body)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .send()
        .await;

    match reqwest_response {
        Ok(res) => match res.json::<serde_json::Value>().await {
            Ok(json) => handle_json_response(json),
            Err(_) => response_with_status(StatusCode::BAD_REQUEST, "JSON was not well-formatted"),
        },
        Err(err) => {
            tracing::error!("Request to Ethereum node failed: {:?}", err);
            response_with_status(StatusCode::BAD_REQUEST, "Request failed")
        }
    }
}

fn handle_json_response(json: serde_json::Value) -> Response {
    match json {
        serde_json::Value::Object(map) => {
            if let Some(result) = map.get("result") {
                match result.get("stateRoot") {
                    Some(state_root) => {
                        if state_root.is_null() {
                            response_with_status(StatusCode::BAD_REQUEST, "stateRoot is null")
                        } else {
                            let state_root = state_root.as_str().unwrap();
                            response_with_status(StatusCode::OK, state_root)
                        }
                    }
                    None => response_with_status(StatusCode::BAD_REQUEST, "stateRoot is missing"),
                }
            } else if let Some(error) = map.get("error") {
                response_with_status(
                    StatusCode::BAD_REQUEST,
                    error.get("message").unwrap().as_str().unwrap(),
                )
            } else {
                response_with_status(
                    StatusCode::BAD_REQUEST,
                    "Invalid response from eth_getBlockByNumber API",
                )
            }
        }
        _ => response_with_status(StatusCode::BAD_REQUEST, "JSON was not well-formatted"),
    }
}

fn response_with_status(status: StatusCode, message: &str) -> Response {
    (status, Json(message)).into_response()
}

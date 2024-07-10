use std::str::FromStr;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use dotenv::dotenv;
use ethers::{
    providers::{Http, Middleware, Provider},
    types::{BlockId, BlockNumber},
};
use primitive_types::H256;
use reqwest::Client;
use serde_json;

use crate::model::eth_rpc::Input;

pub async fn call_mev_blocker_api(
    State(_): State<Client>,
    Json(input): Json<Input>,
) -> impl IntoResponse {
    dotenv().ok();
    let account_address = input.account_address;
    let storage_keys = input.storage_keys;

    // let body = EthRpcGetProofBody {
    //     jsonrpc: "2.0".to_string(),
    //     method: "eth_getProof".to_string(),
    //     params: vec![
    //         EthRpcGetProofBodyParams::AccountAddress(account_address),
    //         EthRpcGetProofBodyParams::StorageKeys(storage_keys.clone()),
    //         EthRpcGetProofBodyParams::BlockIdentifier("latest".to_string()),
    //     ],
    //     id: "1".to_string(),
    // };

    // println!("body: {}", serde_json::to_string_pretty(&body).unwrap());
    let url = dotenv::var("ETH_RPC").expect("ETH_RPC must be set");

    let provider = Provider::<Http>::try_from(&url).unwrap();
    let reqwest_response = provider
        .get_proof(
            account_address,
            storage_keys
                .iter()
                .map(|key| H256::from_str(key).unwrap())
                .collect(),
            Some(BlockId::Number(BlockNumber::Latest)),
        )
        .await;
    //
    // let reqwest_response = client
    //     .post(&url)
    //     .json(&body)
    //     .header("Content-Type", "application/json")
    //     .header("Accept", "application/json")
    //     .send()
    //     .await;
    //
    println!("reqwest_response: {:?}", reqwest_response);

    match reqwest_response {
        Ok(res) => (StatusCode::OK, Json(res)).into_response(),
        Err(_) => response_with_status(StatusCode::INTERNAL_SERVER_ERROR, "Request failed"),
    }
}

fn response_with_status(status: StatusCode, message: &str) -> Response {
    (status, Json(message)).into_response()
}

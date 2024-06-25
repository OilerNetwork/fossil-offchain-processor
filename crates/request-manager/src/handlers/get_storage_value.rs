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
use proof_generator::model::eth_rpc::{EthRpcBody, EthRpcBodyParams};

#[derive(Deserialize)]
pub struct StorageRequest {
    pub account_address: String,
    pub storage_key: String,
}

pub(crate) async fn get_storage_value(
    State(app_state): State<AppState>,
    Json(input): Json<StorageRequest>,
) -> impl IntoResponse {
    todo!();
}

async fn get_block_hash_from_dispatcher(
    client: &Client,
    account_address: &str,
) -> Result<String, ()> {
    todo!();
}

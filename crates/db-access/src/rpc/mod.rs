mod utils;

use crate::rpc::utils::json_to_block_header;
use dotenv::dotenv;
use eth_rlp_verify::block_header::BlockHeader;
use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use eyre::Result;
use tracing::info;

pub async fn get_block_by_number(block_number: u64) -> Result<BlockHeader> {
    dotenv().ok();

    let rpc_url = env::var("ETH_RPC_URL").expect("ETH_RPC_URL must be set in .env");

    let block = format!("0x{:x}", block_number);

    let data = json!({
        "jsonrpc": "2.0",
        "method": "eth_getBlockByNumber",
        "params": [block, false],
        "id": 1
    });

    let client = Client::new();

    let response = client
        .post(&rpc_url)
        .header("Content-Type", "application/json")
        .json(&data)
        .send()
        .await?;

    let result: Value = response.json().await?;

    let block_header = json_to_block_header(&result["result"]);

    Ok(block_header)
}

pub async fn get_block_headers_in_range(
    from_block: u64,
    to_block: u64,
) -> Result<Vec<BlockHeader>> {
    dotenv().ok();

    let rpc_url = env::var("ETH_RPC_URL").expect("ETH_RPC_URL must be set in .env");
    let client = Client::new();
    let mut block_headers = Vec::new();

    info!("Fetching block headers from {} to {}", from_block, to_block);

    let mut fetched_block_count = 0;

    for block_number in from_block..=to_block {
        let block = format!("0x{:x}", block_number);

        let data = json!({
            "jsonrpc": "2.0",
            "method": "eth_getBlockByNumber",
            "params": [block, false],
            "id": 1
        });

        let response = client
            .post(&rpc_url)
            .header("Content-Type", "application/json")
            .json(&data)
            .send()
            .await?;

        let result: Value = response.json().await?;

        // Directly convert the JSON result to BlockHeader
        if result["result"].is_object() {
            let block_header = json_to_block_header(&result["result"]);
            block_headers.push(block_header);
            fetched_block_count += 1;
        }
        if fetched_block_count % 64 == 0 && fetched_block_count != 0 {
            info!("Fetched {} block headers", fetched_block_count);
        }
    }

    Ok(block_headers)
}

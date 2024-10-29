mod utils;

use crate::rpc::utils::json_to_block_header;
use alloy_chains::Chain;
use dotenv::dotenv;
use eth_rlp_verify::block_header::BlockHeader;
use eyre::Result;
use foundry_block_explorers::Client as EtherscanClient;
use reqwest::Client;
use serde_json::{json, Value};
use std::env;

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

pub async fn get_block_headers_by_time_range(
    start_timestamp: u64,
    end_timestamp: u64,
) -> Result<Vec<BlockHeader>> {
    let start_block_number = get_block_number_by_timestamp(start_timestamp).await?;
    let end_block_number = get_block_number_by_timestamp(end_timestamp).await?;
    tracing::info!("Start block number: {}", start_block_number);
    tracing::info!("End block number: {}", end_block_number);

    get_block_headers_in_range(start_block_number, end_block_number).await
}

pub async fn get_block_headers_in_range(
    from_block: u64,
    to_block: u64,
) -> Result<Vec<BlockHeader>> {
    dotenv().ok();

    let rpc_url = env::var("ETH_RPC_URL").expect("ETH_RPC_URL must be set in .env");
    let client = Client::new();
    let mut block_headers = Vec::new();

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
        }
    }

    Ok(block_headers)
}

pub async fn get_block_number_by_timestamp(timestamp: u64) -> Result<u64> {
    tracing::debug!("Getting block number by timestamp: {}", timestamp);
    let client = EtherscanClient::new(Chain::mainnet(), "S2161TQ7QZ13XUV4PJ5NIPCDQ2W2IYUJS6")?;
    let response = client.get_block_by_timestamp(timestamp, "before").await?;

    let block_number = response
        .block_number
        .as_number()
        .expect("Block number is not a number")
        .try_into()
        .expect("Failed to convert block number");

    tracing::debug!("Block number: {}", block_number);

    Ok(block_number)
}

pub fn filter_headers(all_headers: &[BlockHeader], range: (u64, u64)) -> Vec<BlockHeader> {
    all_headers
        .iter()
        .filter_map(|header| {
            header.timestamp.as_ref().and_then(|ts| {
                u64::from_str_radix(ts.trim_start_matches("0x"), 16).ok()
            }).and_then(|timestamp| {
                if timestamp >= range.0 && timestamp <= range.1 {
                    Some(header.clone())
                } else {
                    None
                }
            })
        })
        .collect()
}

pub fn get_largest_time_range(ranges: &[(u64, u64)]) -> (u64, u64) {
    ranges.iter().fold(ranges[0], |acc, &range| {
        let acc_duration = acc.1 - acc.0;
        let range_duration = range.1 - range.0;
        if range_duration > acc_duration {
            range
        } else {
            acc
        }
    })
}

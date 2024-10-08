use dotenv::dotenv;
use eth_rlp_verify::block_header::BlockHeader;
use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use std::error::Error;

// Helper function to parse hex to i64
fn parse_hex_to_i64(hex_str: &str) -> Option<i64> {
    i64::from_str_radix(hex_str.trim_start_matches("0x"), 16).ok()
}

// The function now returns a `BlockHeader` struct
pub async fn get_block_by_number(block_number: u64) -> Result<BlockHeader, Box<dyn Error>> {
    // Load .env variables
    dotenv().ok();

    // Get the ETH_RPC_URL from the environment
    let rpc_url = env::var("ETH_RPC_URL").expect("ETH_RPC_URL must be set in .env");

    // Format the block number as a hex string
    let block = format!("0x{:x}", block_number);

    // Create the JSON-RPC payload
    let data = json!({
        "jsonrpc": "2.0",
        "method": "eth_getBlockByNumber",
        "params": [block, false],
        "id": 1
    });

    // Create a reqwest client
    let client = Client::new();

    // Send the request and get the response
    let response = client
        .post(&rpc_url)
        .header("Content-Type", "application/json")
        .json(&data)
        .send()
        .await?;

    // Parse the JSON response
    let result: Value = response.json().await?;

    // Extract the block result from the JSON-RPC response
    let block_result = &result["result"];

    // Map the fields from the response into the BlockHeader struct
    let block_header = BlockHeader {
        block_hash: block_result["hash"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        number: parse_hex_to_i64(block_result["number"].as_str().unwrap_or_default())
            .unwrap_or_default(),
        gas_limit: parse_hex_to_i64(block_result["gasLimit"].as_str().unwrap_or_default())
            .unwrap_or_default(),
        gas_used: parse_hex_to_i64(block_result["gasUsed"].as_str().unwrap_or_default())
            .unwrap_or_default(),
        nonce: block_result["nonce"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        transaction_root: block_result["transactionsRoot"]
            .as_str()
            .map(|s| s.to_string()),
        receipts_root: block_result["receiptsRoot"].as_str().map(|s| s.to_string()),
        state_root: block_result["stateRoot"].as_str().map(|s| s.to_string()),
        base_fee_per_gas: block_result["baseFeePerGas"]
            .as_str()
            .map(|s| s.to_string()),
        parent_hash: block_result["parentHash"].as_str().map(|s| s.to_string()),
        ommers_hash: block_result["sha3Uncles"].as_str().map(|s| s.to_string()),
        miner: block_result["miner"].as_str().map(|s| s.to_string()),
        logs_bloom: block_result["logsBloom"].as_str().map(|s| s.to_string()),
        difficulty: block_result["difficulty"].as_str().map(|s| s.to_string()),
        totaldifficulty: block_result["totalDifficulty"]
            .as_str()
            .map(|s| s.to_string()),
        sha3_uncles: block_result["sha3Uncles"].as_str().map(|s| s.to_string()),
        timestamp: block_result["timestamp"].as_str().map(|s| s.to_string()),
        extra_data: block_result["extraData"].as_str().map(|s| s.to_string()),
        mix_hash: block_result["mixHash"].as_str().map(|s| s.to_string()),
        withdrawals_root: block_result["withdrawalsRoot"]
            .as_str()
            .map(|s| s.to_string()),
        blob_gas_used: None,   // Not present in the JSON result you provided
        excess_blob_gas: None, // Not present in the JSON result you provided
        parent_beacon_block_root: None, // Not present in the JSON result you provided
    };

    // Return the populated BlockHeader
    Ok(block_header)
}

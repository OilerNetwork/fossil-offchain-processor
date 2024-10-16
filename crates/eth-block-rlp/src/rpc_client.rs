use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::error::Error;

/// Fetches the block header from the Ethereum blockchain using the given RPC URL and block number.
///
/// # Arguments
///
/// * `rpc_url` - The URL of the Ethereum RPC endpoint.
/// * `block_number` - The block number in hexadecimal format.
pub async fn fetch_block_header<T: DeserializeOwned>(
    rpc_url: &str,
    block_number: &str,
) -> Result<(String, T), Box<dyn Error>> {
    let client = Client::new();
    let response = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getBlockByNumber",
            "params": [block_number, false],
            "id": 1,
        }))
        .send()
        .await?
        .json::<Value>()
        .await?;

    let block_hash = response["result"]["hash"]
        .as_str()
        .ok_or("Missing block hash")?
        .to_string();
    let rpc_header: T = serde_json::from_value(response["result"].clone())?;
    Ok((block_hash, rpc_header))
}

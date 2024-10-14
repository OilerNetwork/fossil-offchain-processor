use eth_rlp_verify::block_header::BlockHeader;
use serde_json::Value;

fn parse_hex_to_i64(hex_str: &str) -> Option<i64> {
    i64::from_str_radix(hex_str.trim_start_matches("0x"), 16).ok()
}

pub fn json_to_block_header(block_result: &Value) -> BlockHeader {
    BlockHeader {
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

        // Wrap in Some for Option<String>
        logs_bloom: Some(
            block_result["logsBloom"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_default(),
        ), // Default to an empty bloom if not present
        difficulty: Some(
            block_result["difficulty"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or("0x0".to_string()),
        ), // Default to "0x0"
        totaldifficulty: Some(
            block_result["totalDifficulty"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or("0x0".to_string()),
        ), // Default to "0x0"
        sha3_uncles: block_result["sha3Uncles"].as_str().map(|s| s.to_string()),
        timestamp: block_result["timestamp"].as_str().map(|s| s.to_string()), // Option<String>
        extra_data: Some(
            block_result["extraData"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_default(),
        ), // Default to an empty string if not present
        mix_hash: Some(
            block_result["mixHash"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_default(),
        ), // Default to an empty string if not present
        withdrawals_root: Some(
            block_result["withdrawalsRoot"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_default(),
        ), // Default to an empty string if not present
        blob_gas_used: Some(
            block_result["blobGasUsed"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_default(),
        ), // Default to an empty string if not present
        excess_blob_gas: Some(
            block_result["excessBlobGas"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_default(),
        ), // Default to an empty string if not present
        parent_beacon_block_root: Some(
            block_result["parentBeaconBlockRoot"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_default(),
        ), // Default to an empty string if not present
    }
}

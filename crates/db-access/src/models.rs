use eth_rlp_types::BlockHeader as EthBlockHeader;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(sqlx::FromRow, Debug)]
pub struct BlockHeader {
    pub block_hash: Option<String>,
    pub number: i64,
    pub gas_limit: Option<i64>,
    pub gas_used: Option<i64>,
    pub base_fee_per_gas: Option<String>,
    pub nonce: Option<String>,
    pub transaction_root: Option<String>,
    pub receipts_root: Option<String>,
    pub state_root: Option<String>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Transaction {
    pub block_number: Option<i64>,
    pub transaction_hash: String,
    pub transaction_index: Option<i32>,
    pub from_addr: Option<String>,
    pub to_addr: Option<String>,
    pub value: Option<String>,
    pub gas_price: Option<String>,
    pub max_priority_fee_per_gas: Option<String>,
    pub max_fee_per_gas: Option<String>,
    pub gas: Option<String>,
    pub chain_id: Option<String>,
}

#[derive(Debug)]
pub struct BlockHeaderSubset {
    pub number: i64,
    pub base_fee_per_gas: Option<String>,
    pub timestamp: Option<i64>,
}

#[derive(sqlx::FromRow, Debug)]
pub struct ApiKey {
    pub key: String,
    pub name: Option<String>,
}

#[derive(sqlx::Type, Debug, PartialEq, Serialize, Deserialize)]
#[sqlx(type_name = "TEXT")]
pub enum JobStatus {
    Pending,
    Completed,
    Failed,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "Pending"),
            JobStatus::Completed => write!(f, "Completed"),
            JobStatus::Failed => write!(f, "Failed"),
        }
    }
}

// impl FromStr for JobStatus {
//     type Err = ();

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         match s {
//             "Pending" => Ok(JobStatus::Pending),
//             "Completed" => Ok(JobStatus::Completed),
//             "Failed" => Ok(JobStatus::Failed),
//             _ => Err(()),
//         }
//     }
// }

#[derive(sqlx::FromRow, Debug)]
pub struct JobRequest {
    pub job_id: String,
    pub status: JobStatus,
    pub created_at: chrono::NaiveDateTime,
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TempBlockHeader {
    pub block_hash: String,
    pub number: i64,
    pub gas_limit: i64,
    pub gas_used: i64,
    pub nonce: String,
    pub transaction_root: Option<String>,
    pub receipts_root: Option<String>,
    pub state_root: Option<String>,
    pub base_fee_per_gas: Option<String>,
    pub parent_hash: Option<String>,
    pub miner: Option<String>,
    pub logs_bloom: Option<String>,
    pub difficulty: Option<String>,
    pub totaldifficulty: Option<String>,
    pub sha3_uncles: Option<String>,
    pub timestamp: Option<i64>, // Assuming this is stored as bigint
    pub extra_data: Option<String>,
    pub mix_hash: Option<String>,
    pub withdrawals_root: Option<String>,
    pub blob_gas_used: Option<String>,
    pub excess_blob_gas: Option<String>,
    pub parent_beacon_block_root: Option<String>,
}

// fn parse_hex_to_i64(hex_str: &str) -> Option<i64> {
//     i64::from_str_radix(hex_str.trim_start_matches("0x"), 16).ok()
// }

pub fn temp_to_block_header(temp: TempBlockHeader) -> EthBlockHeader {
    EthBlockHeader {
        block_hash: temp.block_hash,             // String (not Option<String>)
        number: temp.number,                     // i64 (not Option<i64>)
        gas_limit: temp.gas_limit,               // i64 (not Option<i64>)
        gas_used: temp.gas_used,                 // i64 (not Option<i64>)
        nonce: temp.nonce,                       // String (not Option<String>)
        transaction_root: temp.transaction_root, // Option<String>
        receipts_root: temp.receipts_root,       // Option<String>
        state_root: temp.state_root,             // Option<String>
        base_fee_per_gas: temp.base_fee_per_gas, // Option<String>

        // Only assign fields that exist in EthBlockHeader
        parent_hash: temp.parent_hash, // Option<String> (if exists)
        ommers_hash: temp.sha3_uncles.clone(), // Option<String> (if exists)
        miner: temp.miner,             // Option<String> (if exists)

        // For the following, use Option<String> correctly
        logs_bloom: Some(temp.logs_bloom.unwrap_or_default()),
        difficulty: Some(temp.difficulty.unwrap_or_else(|| "0x0".to_string())),
        totaldifficulty: Some(temp.totaldifficulty.unwrap_or_else(|| "0x0".to_string())),
        sha3_uncles: temp.sha3_uncles, // Option<String> (if exists)

        // Convert timestamp from Option<i64> to Option<String>
        timestamp: temp.timestamp.map(|ts| format!("0x{:x}", ts)), // Convert i64 to hex string
        extra_data: Some(temp.extra_data.unwrap_or_default()),
        mix_hash: Some(temp.mix_hash.unwrap_or_default()),
        withdrawals_root: Some(temp.withdrawals_root.unwrap_or_default()),
        blob_gas_used: Some(temp.blob_gas_used.unwrap_or_default()),
        excess_blob_gas: Some(temp.excess_blob_gas.unwrap_or_default()),
        parent_beacon_block_root: Some(temp.parent_beacon_block_root.unwrap_or_default()),
    }
}

use serde::{Deserialize, Serialize};

use super::hex::HexString;

#[derive(Deserialize)]
pub struct Input {
    pub account_address: String,
    pub storage_keys: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct EthRpcGetProofBody {
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<EthRpcGetProofBodyParams>,
    pub id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum EthRpcGetProofBodyParams {
    AccountAddress(String),
    StorageKeys(Vec<String>),
    BlockIdentifier(String),
}

#[derive(Serialize, Deserialize)]
pub struct EthRpcGetBlockByNumberBody {
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<EthRpcGetBlockByNumberBodyParams>,
    pub id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum EthRpcGetBlockByNumberBodyParams {
    BlockIdentifier(String),
    IncludeTransactions(bool),
}

#[derive(Deserialize, Debug)]
pub struct BlockNumber {
    pub block_number: HexString,
}

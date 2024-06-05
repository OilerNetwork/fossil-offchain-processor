use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Input {
    pub account_address: String,
    pub storage_keys: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct EthRpcBody {
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<EthRpcBodyParams>,
    pub id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum EthRpcBodyParams {
    AccountAddress(String),
    StorageKeys(Vec<String>),
    BlockIdentifier(String),
}

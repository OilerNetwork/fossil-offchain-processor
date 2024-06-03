use primitive_types::U256;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountProof {
    pub address: U256,
    pub balance: U256,
    pub code_hash: U256,
    pub nonce: u64,
    pub storage_hash: U256,
    pub bytes: Vec<usize>,
    pub data: Vec<u64>,
}

use primitive_types::U256;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageProof {
    pub key: U256,
    pub bytes: Vec<usize>,
    pub data: Vec<u64>,
    pub value: U256,
}

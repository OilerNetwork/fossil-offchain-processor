use serde::{Deserialize, Serialize};

use crate::model::account_proof::AccountProof;
use crate::model::storage_proof::StorageProof;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proof {
    pub account_proof: AccountProof,
    pub storage_proof: StorageProof,
    pub len_proof: usize,
    pub state_root: String,
}

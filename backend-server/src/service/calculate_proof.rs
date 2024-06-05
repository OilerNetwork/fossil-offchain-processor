use std::str::FromStr;

use primitive_types::U256;
use serde::Deserialize;
use serde_json;

use crate::model::account_proof::AccountProof;
use crate::model::data::{Data, IntsSequence};
use crate::model::errors::ProofError;
use crate::model::hex::{u64_or_hex, HexString};
use crate::model::proof::Proof;
use crate::model::storage_proof::StorageProof;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EthTrieProofs {
    address: U256,
    balance: U256,
    code_hash: U256,
    #[serde(deserialize_with = "u64_or_hex")]
    nonce: u64,
    storage_hash: U256,
    account_proof: Vec<String>,
    storage_proof: Vec<EthStorageProof>,
}

#[derive(Deserialize)]
struct EthStorageProof {
    key: U256,
    value: U256,
    proof: Vec<String>,
}

pub fn calculate_proof(
    trie_proof: &serde_json::Value,
    storage_keys: &[String],
) -> Result<Proof, ProofError> {
    let eth_trie_proofs: EthTrieProofs = serde_json::from_value(trie_proof.to_owned())?;

    if eth_trie_proofs.account_proof.is_empty() {
        return Err(ProofError::AccountProofEmpty);
    }

    let account_proof_ints_sequence: Vec<IntsSequence> = eth_trie_proofs
        .account_proof
        .iter()
        .map(|element| -> IntsSequence { Data::from(HexString::new(element)).into() })
        .collect();

    let (flat_account_proof, flat_account_proof_sized_bytes) =
        flatten_proof(account_proof_ints_sequence);

    let state_root: U256 = U256::from_str(&storage_keys[0])
        .map_err(|err| ProofError::FromHexError(err.to_string()))?;

    let len_proof = flat_account_proof.len();

    let account_proof = AccountProof {
        address: eth_trie_proofs.address,
        balance: eth_trie_proofs.balance,
        code_hash: eth_trie_proofs.code_hash,
        storage_hash: eth_trie_proofs.storage_hash,
        nonce: eth_trie_proofs.nonce,
        bytes: flat_account_proof_sized_bytes,
        data: flat_account_proof,
    };

    let eth_storage_proof = eth_trie_proofs
        .storage_proof
        .first()
        .ok_or(ProofError::StorageProofEmpty)?;

    let storage_proof_ints_sequence: Vec<IntsSequence> = eth_storage_proof
        .proof
        .iter()
        .map(|element| -> IntsSequence { Data::from(HexString::new(element)).into() })
        .collect();

    let (flat_storage_proof, flat_storage_proof_sized_bytes) =
        flatten_proof(storage_proof_ints_sequence);

    let storage_proof = StorageProof {
        key: eth_storage_proof.key,
        value: eth_storage_proof.value,
        bytes: flat_storage_proof_sized_bytes,
        data: flat_storage_proof,
    };

    Ok(Proof {
        account_proof,
        storage_proof,
        len_proof,
        state_root: state_root.to_string(),
    })
}

fn flatten_proof(proof_sequence: Vec<IntsSequence>) -> (Vec<u64>, Vec<usize>) {
    let mut flat_proof = Vec::new();
    let mut flat_proof_sized_bytes = Vec::new();

    for proof_element in proof_sequence.into_iter() {
        flat_proof.extend(proof_element.values);
        flat_proof_sized_bytes.push(proof_element.length);
    }

    (flat_proof, flat_proof_sized_bytes)
}

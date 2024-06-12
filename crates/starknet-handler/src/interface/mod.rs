use proof_generator::model::{account_proof::AccountProof, storage_proof::StorageProof};

pub fn store_state_root(state_root: String) {
    tracing::info!(?state_root, "Storing state root");
    todo!();
}

pub fn check_account_proof_status(account_proof: AccountProof) {
    tracing::info!(?account_proof, "Checking account proof status");
    todo!();
}

pub fn verify_account_proof(account_proof: AccountProof) {
    tracing::info!(?account_proof, "Verifying account proof");
    todo!();
}

pub fn verify_storage_proof(storage_proof: StorageProof) {
    tracing::info!(?storage_proof, "Checking storage proof status");
    todo!();
}

pub fn get_storage(
    block: u64,
    account: primitive_types::H160,
    slot: primitive_types::H256,
    proof_sizes_bytes: Vec<usize>,
    proofs_concat: Vec<u64>,
    state_root: String,
) -> Option<primitive_types::H256> {
    tracing::info!(
        ?block,
        ?account,
        ?slot,
        ?proof_sizes_bytes,
        ?proofs_concat,
        ?state_root,
        "Getting storage"
    );
    todo!();
}

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

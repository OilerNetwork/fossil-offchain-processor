use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProofError {
    #[error("Account proof is empty")]
    AccountProofEmpty,
    #[error("Storage proof is empty")]
    StorageProofEmpty,
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Hex parse error: {0}")]
    FromHexError(String),
}

impl From<serde_json::Error> for ProofError {
    fn from(err: serde_json::Error) -> ProofError {
        ProofError::ParseError(err.to_string())
    }
}

impl From<hex::FromHexError> for ProofError {
    fn from(err: hex::FromHexError) -> ProofError {
        ProofError::FromHexError(err.to_string())
    }
}

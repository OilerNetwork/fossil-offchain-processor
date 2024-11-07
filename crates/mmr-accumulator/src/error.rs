use thiserror::Error;

#[derive(Debug, Error)]
pub enum MMRProcessorError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("RPC error: {0}")]
    RpcError(#[from] reqwest::Error),

    #[error("MMR error: {0}")]
    MmrError(#[from] mmr::MMRError), // Handle MMR-specific errors

    #[error("Block hash mismatch: expected {expected:?}, got {actual:?}")]
    BlockHashMismatch { expected: String, actual: String },

    #[error("Inconsistent block hashes between chunks: {expected:?} != {actual:?}")]
    InconsistentBlockHash { expected: String, actual: String },

    #[error("Invalid block headers fetched")]
    InvalidBlockHeaders,

    #[error("No block headers fetched")]
    NoBlockHeadersFetched,

    #[error(transparent)]
    Other(#[from] eyre::Report),
}

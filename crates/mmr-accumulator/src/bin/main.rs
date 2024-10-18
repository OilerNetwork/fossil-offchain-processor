use accumulators::{
    hasher::stark_poseidon::StarkPoseidonHasher,
    mmr::MMR,
    store::{sqlite::SQLiteStore, SubKey},
};
use eyre::Result;
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let db_path = "db-instances/0.db";
    let absolute_db_path = std::fs::canonicalize(db_path)?;
    info!("Using database file at: {}", absolute_db_path.display());

    // Connect to the SQLite database
    let pool = SqlitePool::connect(db_path).await?;

    // Query the `mmr_metadata` table to retrieve the mmr_id
    let mmr_id = sqlx::query_scalar::<_, String>("SELECT mmr_id FROM mmr_metadata LIMIT 1")
        .fetch_optional(&pool)
        .await?;

    let mmr_id = match mmr_id {
        Some(id) => {
            info!("Retrieved MMR ID from mmr_metadata: {}", id);
            Some(id)
        }
        None => {
            error!("No MMR ID found in mmr_metadata, using a new MMR ID.");
            None // Generate a new MMR ID if none is found
        }
    };

    // Initialize the SQLite store directly
    let store = Arc::new(SQLiteStore::new(db_path, Some(true), None).await?);

    // Initialize the MMR with the store and Poseidon hasher
    let hasher = Arc::new(StarkPoseidonHasher::new(Some(false)));
    let mmr = MMR::new(store, hasher, mmr_id);

    // Log internal MMR state: element count, leaves count, and root hash
    let elements_count = mmr.elements_count.get().await?;
    let leaves_count = mmr.leaves_count.get().await?;

    // Use SubKey::None for the root hash retrieval
    let root_hash = mmr.root_hash.get(SubKey::None).await?;

    info!("MMR state:");
    info!("Elements count: {}", elements_count);
    info!("Leaves count: {}", leaves_count);
    info!(
        "Root hash: {}",
        root_hash.unwrap_or_else(|| "None".to_string())
    );

    Ok(())
}

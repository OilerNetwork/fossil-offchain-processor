mod store;

use crate::store::StoreManager;
use accumulators::{hasher::keccak::KeccakHasher, mmr::MMR, store::sqlite::SQLiteStore};
use anyhow::Result;
use db_access::{queries::get_block_hashes_by_block_range, DbConnection};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::fs::{self, File};
use std::path::Path;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the database connection for fetching block hashes
    let db = DbConnection::new().await?;

    // Define the block range
    let batch_size = 1024;
    let mut start_block = 1;

    // Ensure the `db-instances/` directory exists, create if not
    let db_dir = "db-instances";
    let current_dir = env::current_dir()?.join(db_dir);
    if !current_dir.exists() {
        println!("Creating directory: {}", current_dir.display());
        fs::create_dir_all(&current_dir)?; // Ensure directory is created
    } else {
        println!("Directory already exists: {}", current_dir.display());
    }

    // Loop through block ranges to create multiple MMRs
    let mut db_file_counter = 0; // Start the file name counter from 0.db

    while let Some(block_hashes) = get_next_block_range(&db, start_block, batch_size).await? {
        if block_hashes.is_empty() {
            break; // Exit if no more blocks are available
        }

        let end_block = start_block + batch_size - 1;
        println!("Processing blocks {} to {}", start_block, end_block);

        // Create a new SQLite database file in `db-instances/` with an absolute path
        let store_path = current_dir.join(format!("{}.db", db_file_counter));
        let store_path_str = store_path.to_str().unwrap(); // Convert Path to &str
        println!("Creating database file at: {}", store_path_str);

        // If the file doesn't exist, create it manually
        if !Path::new(store_path_str).exists() {
            println!("Creating empty database file: {}", store_path_str);
            File::create(store_path_str)?; // This ensures the file is created
        }

        db_file_counter += 1; // Increment for the next batch

        // Ensure the SQLite database file can be created
        let store_manager = StoreManager::new(&store_path_str).await?;
        let pool = SqlitePool::connect(&store_path_str).await?;
        let store = Arc::new(SQLiteStore::new(&store_path_str, Some(true), None).await?);

        // Initialize the MMR for this block range
        let hasher = Arc::new(KeccakHasher::new());
        let mut mmr = MMR::new(store, hasher, None);

        // Append each block hash to the MMR
        for hash in block_hashes.iter() {
            let append_result = mmr.append(hash.clone()).await?;
            store_manager
                .insert_value_index_mapping(&pool, hash, append_result.element_index)
                .await?;
        }

        // Move to the next batch of blocks
        start_block += batch_size;
    }

    Ok(())
}

// Fetch the next block range from the database
async fn get_next_block_range(
    db: &DbConnection,
    start_block: i64,
    batch_size: i64,
) -> Result<Option<Vec<String>>, sqlx::Error> {
    let end_block = start_block + batch_size - 1;
    let block_hashes = get_block_hashes_by_block_range(&db.pool, start_block, end_block).await?;

    if block_hashes.is_empty() {
        Ok(None) // No more blocks to process
    } else {
        Ok(Some(block_hashes))
    }
}

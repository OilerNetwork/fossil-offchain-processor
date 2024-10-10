use crate::store::StoreManager;
use accumulators::{
    hasher::stark_poseidon::StarkPoseidonHasher, mmr::MMR, store::sqlite::SQLiteStore,
};
use anyhow::Result;
use db_access::{queries::get_block_hashes_by_block_range, DbConnection};
use sqlx::{Row, SqlitePool};
use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Ensures that a directory exists, creates it if necessary
pub fn ensure_directory_exists(dir_name: &str) -> Result<PathBuf> {
    let current_dir = env::current_dir()?.join(dir_name);
    if !current_dir.exists() {
        info!("Creating directory: {}", current_dir.display());
        fs::create_dir_all(&current_dir)?; // Ensure directory is created
    } else {
        info!("Directory already exists: {}", current_dir.display());
    }
    Ok(current_dir)
}

/// Creates a database file if it doesn't exist and returns the path to the file
pub fn create_database_file(current_dir: &Path, db_file_counter: usize) -> Result<String> {
    let store_path = current_dir.join(format!("{}.db", db_file_counter));
    let store_path_str = store_path.to_str().unwrap();

    if !Path::new(store_path_str).exists() {
        info!("Creating empty database file: {}", store_path_str);
        File::create(store_path_str)?; // Ensure the file is created
    }

    Ok(store_path_str.to_string())
}

/// Initializes the MMR by retrieving or creating the MMR ID and setting up the hasher and store
pub async fn initialize_mmr(store_path: &str) -> Result<(StoreManager, MMR, SqlitePool)> {
    let pool = SqlitePool::connect(store_path).await?;
    let store_manager = StoreManager::new(store_path).await?;
    let store = Arc::new(SQLiteStore::new(store_path, Some(true), None).await?);

    // Retrieve or generate a new MMR ID
    let mmr_id = match get_mmr_id(&pool).await? {
        Some(id) => id,
        None => {
            let new_id = Uuid::new_v4().to_string();
            save_mmr_id(&pool, &new_id).await?;
            new_id
        }
    };
    info!("Using MMR ID: {}", mmr_id);

    let hasher = Arc::new(StarkPoseidonHasher::new(Some(false)));
    let mmr = MMR::new(store, hasher, Some(mmr_id.clone()));

    Ok((store_manager, mmr, pool))
}

/// Fetches the next block range from the database
pub async fn get_next_block_range(
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

/// Retrieves the MMR ID from the `mmr_metadata` table
async fn get_mmr_id(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query("SELECT mmr_id FROM mmr_metadata LIMIT 1")
        .fetch_optional(pool)
        .await?;

    if let Some(row) = row {
        let mmr_id: String = row.get("mmr_id");
        Ok(Some(mmr_id))
    } else {
        Ok(None)
    }
}

/// Saves the MMR ID to the `mmr_metadata` table
async fn save_mmr_id(pool: &SqlitePool, mmr_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR REPLACE INTO mmr_metadata (mmr_id) VALUES (?)")
        .bind(mmr_id)
        .execute(pool)
        .await?;
    Ok(())
}

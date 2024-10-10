use anyhow::Result;
use db_access::DbConnection;
use tracing::{error, info};
use mmr_accumulator::processor_utils::*;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    info!("Starting MMR Processor");

    let db = DbConnection::new().await?;

    // Define the block range and initialize the MMR
    let batch_size = 1024;
    let mut start_block = 1;
    let mut batch_number = 1;
    let mut db_file_counter = 0; // Start the file name counter from 0.db

    // Ensure the `db-instances/` directory exists
    let current_dir = ensure_directory_exists("db-instances")?;

    while let Some(block_hashes) = get_next_block_range(&db, start_block, batch_size).await? {
        if block_hashes.is_empty() {
            info!("No more blocks to process");
            break;
        }

        info!("Processing batch {}: blocks {} to {}", batch_number, start_block, start_block + batch_size - 1);

        // Ensure the SQLite database file can be created
        let store_path = create_database_file(&current_dir, db_file_counter)?;
        let (store_manager, mut mmr, pool) = initialize_mmr(&store_path).await?;

        let mut root_hash: Option<String> = None;

        // Append each block hash to the MMR
        for hash in block_hashes.iter() {
            let append_result = mmr.append(hash.clone()).await?;

            store_manager
                .insert_value_index_mapping(&pool, hash, append_result.element_index)
                .await?;

            // Update the root hash for the current batch
            root_hash = Some(append_result.root_hash.to_string());
        }

        // Log the batch number and the final root hash for this batch
        if let Some(hash) = root_hash {
            info!("Completed batch {}: root hash = {}", batch_number, hash);
        } else {
            error!("No root hash generated for batch {}", batch_number);
        }

        // Increment file counter and process the next block
        db_file_counter += 1;
        start_block += batch_size;
        batch_number += 1;

        if db_file_counter == 4 {
            break;
        }
    }

    info!("MMR processing complete");
    Ok(())
}



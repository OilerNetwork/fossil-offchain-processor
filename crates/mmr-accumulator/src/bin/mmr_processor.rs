use block_validity::utils::are_blocks_and_chain_valid;
use db_access::rpc::get_block_headers_in_range;
// use db_access::DbConnection;
use eyre::{ContextCompat, Result};
use mmr_accumulator::error::MMRProcessorError;
use mmr_accumulator::ethereum::get_finalized_block_hash;
use mmr_accumulator::processor_utils::*;
use tracing::info;

const BATCH_SIZE: u64 = 8;

#[tokio::main]
async fn main() -> Result<(), MMRProcessorError> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting MMR Processor");

    let (finalized_block_number, finalized_block_hash) = get_finalized_block_hash().await?;
    info!("Finalized block number: {}", finalized_block_number);

    // let db = DbConnection::new().await?;

    // Initialize MMR
    let db_file = 0;
    let current_dir = ensure_directory_exists("db-instances")?;
    let store_path = create_database_file(&current_dir, db_file)?;
    let (store_manager, mut mmr, pool) = initialize_mmr(&store_path).await?;

    // Process 10 batches of 1024 blocks
    let mut batch_last_block_number = finalized_block_number;
    let mut previous_batch_first_block_hash = None;

    for batch_index in 0..10 {
        info!("Processing batch: {}", batch_index);

        let start_block = batch_last_block_number.saturating_sub(BATCH_SIZE);

        let block_headers =
            get_block_headers_in_range(start_block, batch_last_block_number).await?;
        info!("Fetched block headers: {}", block_headers.len());

        if block_headers.is_empty() {
            return Err(MMRProcessorError::NoBlockHeadersFetched);
        }

        // Check block hash only for the first batch
        if batch_index == 0 {
            let latest_block_hash = block_headers
                .last()
                .context("No block headers fetched")?
                .block_hash
                .clone();

            info!("Latest fetched block hash: {}", latest_block_hash);
            if latest_block_hash != finalized_block_hash {
                return Err(MMRProcessorError::BlockHashMismatch {
                    expected: finalized_block_hash.clone(),
                    actual: latest_block_hash.clone(),
                });
            }
        } else {
            let current_batch_last_block_hash = block_headers
                .last()
                .context("No block headers fetched")?
                .block_hash
                .clone();

            if let Some(prev_hash) = previous_batch_first_block_hash {
                if prev_hash != current_batch_last_block_hash.clone() {
                    return Err(MMRProcessorError::InconsistentBlockHash {
                        expected: prev_hash,
                        actual: current_batch_last_block_hash.clone(),
                    });
                }
            }
        }

        let all_valid = are_blocks_and_chain_valid(&block_headers);
        if !all_valid {
            return Err(MMRProcessorError::InvalidBlockHeaders);
        }

        for header in &block_headers {
            let block_hash = header.block_hash.clone();
            let append_result = mmr.append(block_hash.clone()).await?; // This will work now
            store_manager
                .insert_value_index_mapping(&pool, &block_hash, append_result.element_index)
                .await?;
        }

        previous_batch_first_block_hash = Some(
            block_headers
                .first()
                .context("No block headers fetched")?
                .parent_hash
                .clone()
                .expect("Parent hash is None"),
        );

        batch_last_block_number = start_block - 1;
    }

    info!("MMR processing complete");
    Ok(())
}

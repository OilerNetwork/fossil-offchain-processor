use block_validity::utils::are_blocks_and_chain_valid;
use db_access::rpc::get_block_headers_in_range;
use eyre::{ContextCompat, Result};
use mmr_accumulator::ethereum::get_finalized_block_hash;
use mmr_accumulator::processor_utils::*;
use tracing::{error, info};

const BATCH_SIZE: u64 = 63; // Set to 1024 as per your initial requirement

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting MMR Processor");

    let (finalized_block_number, finalized_block_hash) = get_finalized_block_hash().await?;
    info!("Finalized block number: {}", finalized_block_number);

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

        // Determine the start block for the current batch
        let start_block = batch_last_block_number.saturating_sub(BATCH_SIZE);

        // Fetch the block headers; ensure the range is valid
        let block_headers =
            get_block_headers_in_range(start_block, batch_last_block_number).await?;
        info!("Fetched block headers: {}", block_headers.len());
        info!("Start block: {}", start_block);
        info!("End block: {}", batch_last_block_number);

        // Check that the fetched block hash matches the finalized one only for the first batch
        if batch_index == 0 {
            let latest_block_hash = &block_headers
                .last()
                .context("No block headers fetched")?
                .block_hash;

            info!("Latest fetched block hash: {}", latest_block_hash);
            info!("Expected finalized block hash: {}", finalized_block_hash);

            if latest_block_hash != &finalized_block_hash {
                error!("Latest fetched block hash does not match the finalized block hash!");
                return Err(eyre::eyre!("Block hash mismatch"));
            }
            info!("Latest block hash matches the finalized block hash.");
        } else {
            // For subsequent batches, check the consistency of hashes between chunks
            let current_batch_last_block_hash = &block_headers
                .last()
                .context("No block headers fetched")?
                .block_hash;

            if let Some(prev_hash) = previous_batch_first_block_hash {
                if prev_hash != current_batch_last_block_hash.clone() {
                    error!("Inconsistent block hash between chunks: previous batch's first block parent hash: {:?} does not match current batch's last block hash: {:?}", prev_hash, current_batch_last_block_hash);
                    return Err(eyre::eyre!("Inconsistent block hashes between chunks"));
                }
                info!("Consistent block hash between chunks: previous batch's first block parent hash: {:?} matches current batch's last block hash: {:?}", prev_hash, current_batch_last_block_hash);
            }
        }

        // Check the validity of the fetched block headers
        let all_valid = are_blocks_and_chain_valid(&block_headers);
        if !all_valid {
            error!("Block headers are not valid.");
            return Err(eyre::eyre!("Invalid block headers"));
        }
        info!("All fetched block headers are valid.");

        // Append each valid block hash to the MMR
        for header in &block_headers {
            let block_hash = header.block_hash.clone();
            let append_result = mmr.append(block_hash.clone()).await?;

            store_manager
                .insert_value_index_mapping(&pool, &block_hash, append_result.element_index)
                .await?;
        }

        // Update previous_batch_first_block_hash for the next iteration
        previous_batch_first_block_hash = Some(
            block_headers
                .first()
                .context("No block headers fetched")?
                .parent_hash
                .clone()
                .expect("Parent hash is None"),
        );

        // Update batch_last_block_number for the next iteration
        batch_last_block_number = start_block - 1;
    }

    info!("MMR processing complete");
    Ok(())
}

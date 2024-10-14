use block_validity::utils::are_blocks_and_chain_valid;
use db_access::rpc::get_block_headers_in_range; // Ensure BlockHeader is imported
use eyre::{ContextCompat, Result}; // Import Result and Context for error handling
use mmr_accumulator::ethereum::get_finalized_block_hash;
use mmr_accumulator::processor_utils::*;
use tracing::{error, info}; // Import the utility function to check chain validity

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting MMR Processor");

    let (finalized_block_number, finalized_block_hash) = get_finalized_block_hash().await?;

    // Fetch block headers in the range from finalized_block_number - 1024 to finalized_block_number
    let start_block = finalized_block_number.saturating_sub(16);
    let block_headers = get_block_headers_in_range(start_block, finalized_block_number).await?;

    // Check that the fetched block hash matches the finalized one
    let latest_block_hash = &block_headers
        .last()
        .context("No block headers fetched")?
        .block_hash; // Assumes block_hash is a String

    if latest_block_hash != &finalized_block_hash {
        error!("Latest fetched block hash does not match the finalized block hash!");
        return Err(eyre::eyre!("Block hash mismatch"));
    }
    info!("Latest block hash matches the finalized block hash.");

    // Check the validity of the fetched block headers
    let all_valid = are_blocks_and_chain_valid(&block_headers);
    if !all_valid {
        error!("Block headers are not valid.");
        return Err(eyre::eyre!("Invalid block headers"));
    }
    info!("All fetched block headers are valid.");

    // Initialize MMR
    let db_file_counter = 0; // Start the file name counter from 0.db
    let current_dir = ensure_directory_exists("db-instances")?;
    let store_path = create_database_file(&current_dir, db_file_counter)?;
    let (store_manager, mut mmr, pool) = initialize_mmr(&store_path).await?;

    // Append each valid block hash to the MMR
    for header in block_headers {
        let block_hash = header.block_hash.clone();
        info!("Appending block hash: {}", block_hash);
        let append_result = mmr.append(block_hash.clone()).await?;

        store_manager
            .insert_value_index_mapping(&pool, &block_hash, append_result.element_index)
            .await?;
    }

    info!("MMR processing complete");
    Ok(())
}

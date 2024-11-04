pub mod verify_rpc_block;

use block_validity::utils::are_blocks_and_chain_valid;
use db_access::rpc::get_block_headers_in_range;
use eyre::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let from_block: u64 = 13733852;
    let to_block: u64 = from_block + 9;

    let block_headers = get_block_headers_in_range(from_block, to_block).await?;

    let all_valid = are_blocks_and_chain_valid(&block_headers);
    info!("Result: {}", all_valid);

    Ok(())
}

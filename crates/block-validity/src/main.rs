mod utils;

use db_access::rpc::get_block_headers_in_range;
use std::error::Error;
use tracing::info;
use tracing_subscriber;
use utils::are_blocks_and_chain_valid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let from_block: u64 = 13733852;
    let to_block: u64 = from_block + 9;

    let block_headers = get_block_headers_in_range(from_block, to_block).await?;

    let all_valid = are_blocks_and_chain_valid(block_headers);
    info!("Result: {}", all_valid);

    Ok(())
}

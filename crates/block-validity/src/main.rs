mod utils;

use db_access::queries::get_block_headers_by_block_range;
use std::error::Error;
use tracing::info;
use utils::are_blocks_and_chain_valid;
use serde_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let from_block: u64 = 13733852;
    let to_block: u64 = from_block + 1;

    let connection = db_access::DbConnection::new().await?;

    let block_headers =
        get_block_headers_by_block_range(&connection.pool, from_block as i64, to_block as i64).await?;

    let serialized_block_headers = serde_json::to_string(&block_headers)?;
    println!("Serialized block header: {}", serialized_block_headers);

    let all_valid = are_blocks_and_chain_valid(&block_headers);
    info!("Result: {}", all_valid);

    Ok(())
}

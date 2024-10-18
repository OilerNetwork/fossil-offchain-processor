#![deny(unused_crate_dependencies)]
use dotenv as _;

use db_access::models::BlockHeaderSubset;
use db_access::queries::get_base_fees_between_blocks;
use db_access::DbConnection;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // Connect to DB
    let db = DbConnection::new().await?;

    // Get block headers
    let start_block = 13733852;
    let end_block = start_block + 5;
    let block_headers: Vec<BlockHeaderSubset> =
        get_base_fees_between_blocks(&db.pool, start_block, end_block).await?;

    println!("{:?}", block_headers);
    Ok(())
}

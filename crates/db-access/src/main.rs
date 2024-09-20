use db_access::models::BlockHeaderSubset;
use db_access::queries::get_base_fees_between_blocks;
use db_access::volatility::calculate_volatility;
use db_access::DbConnection;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // Connect to DB
    let db = DbConnection::new().await?;

    // Volatlity

    // Get block headers
    let start_block = 13733852;
    let end_block = start_block + 5;
    let block_headers: Vec<BlockHeaderSubset> =
        get_base_fees_between_blocks(&db.pool, start_block, end_block).await?;

    // Calculate the volatility
    let volatility = calculate_volatility(&block_headers).await;

    println!(
        "> VOL = {:.4}%, {} as u128",
        volatility as f32 / 10_000.0,
        volatility
    );

    // End Volatility

    Ok(())
}

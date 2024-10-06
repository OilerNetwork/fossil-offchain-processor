use db_access::{queries::get_block_by_number, DbConnection};

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let db = DbConnection::new().await?;

    let block_number = 12345;
    match get_block_by_number(&db.pool, block_number).await {
        Ok(Some(block)) => {
            println!("Block found: {:?}", block);
        }
        Ok(None) => {
            println!("No block found with number: {}", block_number);
        }
        Err(e) => {
            eprintln!("Failed to fetch block: {}", e);
        }
    }

    Ok(())
}

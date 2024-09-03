# DB Access Crate example usage

```rust
use db_access::DbConnection;
use db_access::queries::get_base_fees_between_blocks;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // Initialize the database connection
    let db = DbConnection::new().await?;

    // Define the start and end block numbers
    let start_block = 12345;
    let end_block = 14345;

    // Assign the result of the query to a variable
    let block_headers: Vec<BlockHeader> = get_base_fees_between_blocks(&db.pool, start_block, end_block).await?;

    // Now you can use the block_headers variable as needed
    for header in &block_headers {
        println!("Block Number: {}, Base Fee Per Gas: {:?}", header.number, header.base_fee_per_gas);
    }

    // Example of further usage: extracting specific data from the first result
    if let Some(first_header) = block_headers.first() {
        let first_block_number = first_header.number;
        let first_base_fee = &first_header.base_fee_per_gas;
        println!("First Block Number: {}, First Base Fee Per Gas: {:?}", first_block_number, first_base_fee);
    }

    Ok(())
}
```
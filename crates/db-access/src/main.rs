use db_access::rpc::get_block_by_number;
use eth_rlp_verify::verify_block;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let block_number: u64 = 13733852;
    let block = get_block_by_number(block_number).await?;
    let block_hash = block.block_hash.clone();
    let parent_hash = block.parent_hash.clone();

    let is_valid = verify_block(block_number, block, &block_hash);

    let parent_block = get_block_by_number(block_number - 1).await?;
    let parent_block_hash = parent_block.block_hash.clone();

    let is_valid_parent = parent_hash.unwrap() == parent_block_hash;

    println!(
        "The block is valid: {}, and the parent block is valid: {}",
        is_valid, is_valid_parent
    );

    Ok(())
}

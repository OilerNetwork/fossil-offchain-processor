mod rpc;
mod utils;

use crate::rpc::get_block_headers_in_range;
use crate::utils::save_blockheaders_to_file;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let block_headers = get_block_headers_in_range(1, 2).await?;
    let _ = save_blockheaders_to_file(&block_headers, "block_headers.json");

    Ok(())
}

use eth_block_rlp::encode_block_by_number; // Adjust the import to match your module structure

#[tokio::main]
async fn main() {
    let block_number: u64 = 20673165; // Replace with the actual block number

    match encode_block_by_number(block_number).await {
        Ok(encoded_block) => {
            println!("RLP-encoded block: {:?}", encoded_block);
        }
        Err(e) => {
            eprintln!("Failed to encode block: {}", e);
        }
    }
}

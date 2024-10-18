use eth_rlp_verify::block_header::BlockHeader;
use eth_rlp_verify::verify_block;
use tracing::{error, info};

pub fn are_blocks_and_chain_valid(block_headers: &Vec<BlockHeader>) -> bool {
    for (i, block) in block_headers.iter().enumerate() {
        let block_hash = block.block_hash.clone();
        let parent_hash = block.parent_hash.clone().unwrap_or_default();
        let block_number = block.number;

        let is_valid = verify_block(block_number as u64, block, &block_hash);

        if !is_valid {
            error!("Block {} is invalid (hash: {})", block_number, block_hash);
            return false;
        }

        if i != 0 {
            let previous_block = &block_headers[i - 1];
            let previous_block_hash = previous_block.block_hash.clone();

            if parent_hash != previous_block_hash {
                error!(
                    "Block {} parent hash mismatch. Expected: {}, Got: {}",
                    block_number, previous_block_hash, parent_hash
                );
                return false;
            }

            info!(
                "Block {} is valid and links to parent {}",
                block_number, previous_block.number
            );
        } else {
            info!("Block {} is valid", block_number);
        }
    }

    info!("All blocks and the chain are valid");

    true
}

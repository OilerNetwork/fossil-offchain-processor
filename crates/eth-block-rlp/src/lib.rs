pub mod block_header;
mod constants;
pub mod eras;
pub mod rpc_client;

use constants::{
    GENESIS_TO_LONDON_END, LONDON_TO_PARIS_END, LONDON_TO_PARIS_START, PARIS_TO_SHANGHAI_END,
    PARIS_TO_SHANGHAI_START,
};
use dotenv::dotenv;
use eras::{
    encode_genesis_to_london, encode_london_to_paris, encode_paris_to_shanghai,
    encode_shanghai_to_cancun,
};
use rpc_client::fetch_block_header;
use std::env;

pub async fn encode_block_by_number(
    block_number: u64,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    dotenv().ok();
    let rpc_url = env::var("ETH_RPC_URL").expect("ETH_RPC_URL must be set");

    let block_number_hex = format!("0x{:X}", block_number);

    let encoded_block = if (LONDON_TO_PARIS_START..=LONDON_TO_PARIS_END).contains(&block_number) {
        let (_block_hash, rpc_header) =
            fetch_block_header::<eras::RpcBlockHeaderLondonToParis>(&rpc_url, &block_number_hex)
                .await?;
        encode_london_to_paris(rpc_header)
    } else if (PARIS_TO_SHANGHAI_START..=PARIS_TO_SHANGHAI_END).contains(&block_number) {
        let (_block_hash, rpc_header) =
            fetch_block_header::<eras::RpcBlockHeaderParisToShanghai>(&rpc_url, &block_number_hex)
                .await?;
        encode_paris_to_shanghai(rpc_header)
    } else if block_number > PARIS_TO_SHANGHAI_END {
        let (_block_hash, rpc_header) =
            fetch_block_header::<eras::RpcBlockHeaderShanghaiToCancun>(&rpc_url, &block_number_hex)
                .await?;
        encode_shanghai_to_cancun(rpc_header)
    } else if block_number <= GENESIS_TO_LONDON_END {
        let (_block_hash, rpc_header) =
            fetch_block_header::<eras::RpcBlockHeaderGenesisToLondon>(&rpc_url, &block_number_hex)
                .await?;
        encode_genesis_to_london(rpc_header)
    } else {
        return Err("Block number is out of the supported range.".into());
    };

    Ok(encoded_block)
}

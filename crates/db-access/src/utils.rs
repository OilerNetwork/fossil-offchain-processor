use serde_json;
use std::fs::File;
use std::io::Write;
use eth_rlp_verify::block_header::BlockHeader;
use eyre::Result;

pub fn save_blockheaders_to_file(headers: &Vec<BlockHeader>, filename: &str) -> Result<()> {
    let serialized = serde_json::to_string(headers)?;
    let mut file = File::create(filename)?;
    file.write_all(serialized.as_bytes())?;
    Ok(())
}

use eth_rlp_types::BlockHeader;
use eyre::Result;
use serde_json;
use std::fs::File;
use std::io::Write;

pub fn save_blockheaders_to_file(headers: &Vec<BlockHeader>, filename: &str) -> Result<()> {
    let serialized = serde_json::to_string(headers)?;
    let mut file = File::create(filename)?;
    file.write_all(serialized.as_bytes())?;
    Ok(())
}

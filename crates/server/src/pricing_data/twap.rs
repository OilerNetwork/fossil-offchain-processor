use super::utils::prepare_data_frame;
use db_access::models::BlockHeader;
use eyre::{eyre, Result};
use polars::prelude::*;

pub async fn calculate_twap(block_headers: Vec<BlockHeader>) -> Result<f64> {
    let df = prepare_data_frame(block_headers)?;

    let mean = df
        .column("base_fee")?
        .f64()?
        .mean()
        .ok_or_else(|| eyre!("Failed to compute mean"))?;

    Ok(mean)
}

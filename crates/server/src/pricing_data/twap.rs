use super::utils::hex_string_to_f64;
use db_access::models::BlockHeader;
use eyre::{anyhow as err, eyre, Result};
use polars::prelude::*;

pub async fn calculate_twap(block_headers: Vec<BlockHeader>) -> Result<f64> {
    if block_headers.is_empty() {
        return Err(err!("No block headers provided."));
    }

    // Prepare DataFrame
    let mut timestamps = Vec::new();
    let mut base_fees = Vec::new();

    for header in block_headers {
        let timestamp = header
            .timestamp
            .ok_or_else(|| err!("No timestamp in header"))?;
        let base_fee = hex_string_to_f64(
            &header
                .base_fee_per_gas
                .ok_or_else(|| err!("No base fee in header"))?,
        )?;
        timestamps.push(timestamp);
        base_fees.push(base_fee);
    }

    let df = DataFrame::new(vec![
        Series::new("timestamp".into(), timestamps.clone()),
        Series::new("base_fee".into(), base_fees),
    ])?;

    let mean = df
        .column("base_fee")?
        .f64()?
        .mean()
        .ok_or_else(|| eyre!("Failed to compute mean"))?;

    Ok(mean)
}

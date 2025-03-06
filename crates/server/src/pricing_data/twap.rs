use super::utils::hex_string_to_f64;
use db_access::models::BlockHeader;
use eyre::{anyhow, Result};

pub async fn calculate_twap(headers: Vec<BlockHeader>) -> Result<f64> {
    if headers.is_empty() {
        return Err(anyhow!("The provided block headers are empty."));
    }

    let total_base_fee = headers.iter().try_fold(0.0, |acc, header| -> Result<f64> {
        let base_fee = header
            .base_fee_per_gas
            .clone()
            .unwrap_or_else(|| "0x0".to_string());
        let fee = hex_string_to_f64(&base_fee)?;
        Ok(acc + fee)
    })?;

    Ok(total_base_fee / headers.len() as f64)
}

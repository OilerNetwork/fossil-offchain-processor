use db_access::models::BlockHeader;
use eyre::{anyhow, Result};

use super::utils::hex_string_to_f64;

/// Calculates the time weighted average price (TWAP) of the base fee.
/// TODO: handle the unwraps properly, or at least propagate them upwards.
pub async fn calculate_twap(headers: Vec<BlockHeader>) -> Result<f64> {
    let total_base_fee = headers
        .iter()
        .map(|header| {
            let base_fee = match header.base_fee_per_gas.clone() {
                Some(val) => val,
                None => "0x0".to_string(),
            };
            hex_string_to_f64(&base_fee)
        })
        .reduce(|prev, current| prev + current);

    let total_base_fee = match total_base_fee {
        Some(val) => val,
        None => return Err(anyhow!("Failed during calculation of twap.")),
    };

    // Calculate the twap, which in this case we are assuming to have the window of 30days
    // according to the given timestamp range.
    let twap_result = total_base_fee / headers.len() as f64;

    Ok(twap_result)
}

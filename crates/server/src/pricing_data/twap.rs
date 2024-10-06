use super::utils::hex_string_to_f64;
use anyhow::{anyhow, Error};
use db_access::models::BlockHeader;
use tracing::{error, instrument};

// Add #[instrument] to enable tracing for the function
#[instrument(skip(headers))]
pub async fn calculate_twap(headers: Vec<BlockHeader>) -> Result<f64, Error> {
    // Attempt to calculate the total base fee from the headers
    let total_base_fee = headers
        .iter()
        .map(|header| {
            let base_fee = match header.base_fee_per_gas.clone() {
                Some(val) => val,
                None => {
                    error!("Missing base_fee_per_gas for BlockHeader: {:?}", header);
                    "0x0".to_string()
                }
            };
            hex_string_to_f64(&base_fee)
        })
        .reduce(|prev, current| prev + current);

    // Handle the case where total_base_fee is None
    let total_base_fee = match total_base_fee {
        Some(val) => val,
        None => {
            error!("Failed to calculate the total base fee, resulting in None.");
            return Err(anyhow!("Failed during calculation of TWAP."));
        }
    };

    // Ensure there are headers to calculate the TWAP
    if headers.is_empty() {
        error!("No headers provided for TWAP calculation.");
        return Err(anyhow!("Cannot calculate TWAP: No headers provided."));
    }

    // Calculate the TWAP using the total base fee and the number of headers
    let twap_result = total_base_fee / headers.len() as f64;

    // Log the successful TWAP calculation
    tracing::info!("Successfully calculated TWAP: {}", twap_result);

    Ok(twap_result)
}

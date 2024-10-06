use super::utils::hex_string_to_f64;
use anyhow::{anyhow, Error};
use db_access::models::BlockHeader;
use tracing::{error, instrument};

// Add #[instrument] to enable tracing for the function
#[instrument(skip(blocks))]
pub async fn calculate_volatility(blocks: Vec<BlockHeader>) -> Result<f64, Error> {
    let mut returns: Vec<f64> = Vec::new();

    // Calculate log returns for each pair of consecutive block base fees
    for i in 1..blocks.len() {
        if let (Some(ref basefee_current), Some(ref basefee_previous)) =
            (&blocks[i].base_fee_per_gas, &blocks[i - 1].base_fee_per_gas)
        {
            // Convert base fees from hex string to f64
            let basefee_current = match hex_string_to_f64(basefee_current) {
                Ok(value) => value,
                Err(e) => {
                    error!("Failed to convert current base fee from hex to f64: {}", e);
                    return Err(e);
                }
            };

            let basefee_previous = match hex_string_to_f64(basefee_previous) {
                Ok(value) => value,
                Err(e) => {
                    error!("Failed to convert previous base fee from hex to f64: {}", e);
                    return Err(e);
                }
            };

            // If the previous base fee is zero, skip to the next iteration
            if basefee_previous == 0.0 {
                error!("Previous base fee is zero, skipping log return calculation.");
                continue;
            }

            // Calculate log return and add it to the returns vector
            returns.push((basefee_current / basefee_previous).ln());
        }
    }

    // If there are no returns, the volatility is 0
    if returns.is_empty() {
        tracing::info!("No valid returns found, volatility set to 0.");
        return Ok(0f64);
    }

    // Calculate average return
    let mean_return: f64 = returns.iter().sum::<f64>() / returns.len() as f64;

    // Calculate variance of returns
    let variance: f64 = returns
        .iter()
        .map(|&r| (r - mean_return).powi(2))
        .sum::<f64>()
        / returns.len() as f64;

    // Calculate volatility as the square root of the variance and convert to basis points (BPS)
    let volatility_bps = (variance.sqrt() * 10_000.0).round();

    tracing::info!("Volatility calculated successfully: {} BPS", volatility_bps);

    Ok(volatility_bps)
}

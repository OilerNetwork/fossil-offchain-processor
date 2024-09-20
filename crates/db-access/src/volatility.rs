use crate::models::BlockHeaderSubset;

fn hex_string_to_f64(hex_str: &String) -> f64 {
    // Remove the "0x" prefix if it exists
    let stripped = hex_str.trim_start_matches("0x");

    // Return the hex string as f64, panic if it fails
    if let Ok(value) = u128::from_str_radix(stripped, 16) {
        return value as f64;
    } else {
        panic!("Error converting hex string {:?} to f64", hex_str);
    }
}

// Returns volatility as BPS (i.e., 5001 means VOL=50.01%)
pub async fn calculate_volatility(blocks: &[BlockHeaderSubset]) -> u128 {
    // Calculate log returns
    let mut returns: Vec<f64> = Vec::new();
    for i in 1..blocks.len() {
        if let (Some(ref basefee_current), Some(ref basefee_previous)) =
            (&blocks[i].base_fee_per_gas, &blocks[i - 1].base_fee_per_gas)
        {
            // Convert base fees from hex string to f64
            let basefee_current = hex_string_to_f64(&basefee_current);
            let basefee_previous = hex_string_to_f64(&basefee_previous);

            // If the previous base fee is zero, skip to the next iteration
            if basefee_previous == 0.0 {
                continue;
            }

            // Calculate log return and add it to the returns vector
            returns.push((basefee_current / basefee_previous).ln());
        }
    }

    // If there are no returns the volatility is 0
    if returns.is_empty() {
        return 0;
    }

    // Calculate average returns
    let mean_return: f64 = returns.iter().sum::<f64>() / returns.len() as f64;

    // Calculate variance of average returns
    let variance: f64 = returns
        .iter()
        .map(|&r| (r - mean_return).powi(2))
        .sum::<f64>()
        / returns.len() as f64;

    // Square root the variance to get the volatility, translate to BPS (integer)
    (variance.sqrt() * 10_000.0).round() as u128
}

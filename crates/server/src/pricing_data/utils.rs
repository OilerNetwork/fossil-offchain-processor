use anyhow::{anyhow, Error};
use tracing::{error, instrument};

#[instrument(skip(hex_str))]
pub fn hex_string_to_f64(hex_str: &String) -> Result<f64, Error> {
    // Remove the "0x" prefix if it exists
    let stripped = hex_str.trim_start_matches("0x");

    // Try to convert the hex string to u128 and handle errors
    match u128::from_str_radix(stripped, 16) {
        Ok(value) => Ok(value as f64),
        Err(e) => {
            error!("Error converting hex string {:?} to f64: {}", hex_str, e);
            Err(anyhow!(
                "Failed to convert hex string {:?} to f64: {}",
                hex_str,
                e
            ))
        }
    }
}

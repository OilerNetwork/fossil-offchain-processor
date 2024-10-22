use eyre::{anyhow, Result};

pub fn hex_string_to_f64(hex_str: &String) -> Result<f64> {
    // Remove the "0x" prefix if it exists
    let stripped = hex_str.trim_start_matches("0x");

    // Convert hex string to u128, return error if it fails
    u128::from_str_radix(stripped, 16)
        .map(|value| value as f64)
        .map_err(|e| anyhow!("Error converting hex string '{}' to f64: {}", hex_str, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_string_to_f64_zero_value() {
        let result = hex_string_to_f64(&"0x0".to_string());

        assert_eq!(result.unwrap(), 0f64);
    }

    #[test]
    fn test_hex_string_to_f64_prefixed_value() {
        let result = hex_string_to_f64(&"0x12345".to_string());

        assert_eq!(result.unwrap(), 74565_f64);
    }

    #[test]
    fn test_hex_string_to_f64_non_prefixed_value() {
        let result = hex_string_to_f64(&"12345".to_string());

        assert_eq!(result.unwrap(), 74565_f64);
    }

    #[test]
    #[should_panic]
    fn test_hex_string_to_f64_invalid_value() {
        let result = hex_string_to_f64(&"shouldpanic".to_string());

        assert!(result.is_err());
    }
}

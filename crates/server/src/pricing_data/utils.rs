pub fn hex_string_to_f64(hex_str: &String) -> f64 {
    // Remove the "0x" prefix if it exists
    let stripped = hex_str.trim_start_matches("0x");

    // Return the hex string as f64, panic if it fails
    if let Ok(value) = u128::from_str_radix(stripped, 16) {
        value as f64
    } else {
        panic!("Error converting hex string {:?} to f64", hex_str);
    }
}

use num_bigint::BigInt;

use super::data::Data;

pub struct HexString {
    pub hex: String,
}

impl HexString {
    pub fn new(hex_string: &str) -> Self {
        if hex_string.starts_with("0x") {
            Self {
                hex: hex_string.to_string(),
            }
        } else {
            Self {
                hex: format!("0x{}", hex_string),
            }
        }
    }
}

impl From<Data> for HexString {
    fn from(data: Data) -> Self {
        HexString::new(
            data.raw_bytes
                .iter()
                .fold(String::new(), |acc, &x| acc + &format!("{:02x}", x))
                .as_str(),
        )
    }
}

pub fn convert_hex_to_dec(hex_str: &str) -> BigInt {
    let mut result = BigInt::from(0);
    for i in 2..hex_str.len() {
        let digit = BigInt::from(u64::from_str_radix(&hex_str[i..i + 1], 16).unwrap());
        result = result * BigInt::from(16) + digit;
    }
    result
}

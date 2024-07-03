use num_bigint::BigInt;
use serde::{de, Deserialize, Deserializer};

use super::data::Data;

#[derive(Deserialize, Debug)]
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

impl From<HexString> for u64 {
    fn from(hex_string: HexString) -> Self {
        u64::from_str_radix(&hex_string.hex, 16).unwrap()
    }
}

impl From<HexString> for BigInt {
    fn from(hex_string: HexString) -> Self {
        let mut result = BigInt::from(0);
        for i in 2..hex_string.hex.len() {
            let digit = BigInt::from(u64::from_str_radix(&hex_string.hex[i..i + 1], 16).unwrap());
            result = result * BigInt::from(16) + digit;
        }
        result
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum U64OrHex {
    U64(u64),
    Hex(String),
}
pub fn u64_or_hex<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    match U64OrHex::deserialize(deserializer)? {
        U64OrHex::U64(v) => Ok(v),
        U64OrHex::Hex(v) => {
            if v.starts_with("0x") {
                u64::from_str_radix(v.trim_start_matches("0x"), 16)
                    .map_err(|e| de::Error::custom(format!("{}", e)))
            } else {
                Err(de::Error::custom("no 0x prefix"))
            }
        }
    }
}

use std::cmp;

use super::hex;

#[derive(Clone, Debug)]
pub struct Data {
    pub raw_bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct IntsSequence {
    pub values: Vec<u64>,
    pub length: usize,
}

impl Data {
    pub fn new(raw_bytes: Vec<u8>) -> Self {
        Self { raw_bytes }
    }
}

impl From<hex::HexString> for Data {
    fn from(hex_string: hex::HexString) -> Self {
        let raw_bytes = (2..hex_string.hex.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(
                    &hex_string.hex[i..cmp::min(hex_string.hex.len(), i + 2)],
                    16,
                )
                .unwrap()
            })
            .collect();

        Self::new(raw_bytes)
    }
}

fn chunk_bytes_input(input: &[u8], chunk_size: usize) -> Vec<&[u8]> {
    (0..input.len())
        .step_by(chunk_size)
        .map(|i| &input[i..cmp::min(input.len(), i + chunk_size)])
        .collect()
}

impl From<Data> for IntsSequence {
    fn from(data: Data) -> IntsSequence {
        let chunked = chunk_bytes_input(&data.raw_bytes, 8);
        let mut values = Vec::new();
        for chunk in chunked {
            let value = chunk.iter().fold(0, |acc, x| (acc << 8) | *x as u64);
            values.push(value);
        }
        IntsSequence {
            values,
            length: data.raw_bytes.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_hex_string() {
        let hex_string = hex::HexString::new("0x1234567890abcdef");
        let data = Data::from(hex_string);
        assert_eq!(
            data.raw_bytes,
            vec![0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef]
        );
    }

    #[test]
    fn test_into_hex_string() {
        let hex = "0x1234567890abcdef";
        let hex_string = hex::HexString::new(hex);
        let data = Data::from(hex_string);
        let actual_hex_string: hex::HexString = data.into();
        assert_eq!(actual_hex_string.hex, "0x1234567890abcdef");
    }

    #[test]
    fn test_into_ints_sequence() {
        let hex = "0x1234567890abcdef1234567890abcdef";
        let hex_string = hex::HexString::new(hex);
        let data = Data::from(hex_string);
        let actual_ints_sequence: IntsSequence = data.into();
        assert_eq!(
            actual_ints_sequence.values,
            vec![0x1234567890abcdef, 0x1234567890abcdef]
        );
        assert_eq!(actual_ints_sequence.length, 16);
    }
}

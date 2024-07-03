use std::str::FromStr;

use primitive_types::U256;
use starknet::core::types::Felt;

use crate::error::{FieldElementParseError, HandlerError};

pub fn get_high_and_low(state_root: String) -> (u128, u128) {
    let state_root = U256::from_str(state_root.as_str()).unwrap();
    let state_root_low = state_root.low_u128();
    let state_root_high: u128 = (state_root >> 128).as_u128();
    (state_root_high, state_root_low)
}

pub fn prepare_array_data<T: ToString>(data: Vec<T>) -> Result<(Felt, Vec<Felt>), HandlerError> {
    let len = Felt::from_dec_str(data.len().to_string().as_str()).unwrap();
    let data = data
        .iter()
        .map(|d| {
            Felt::from_dec_str(d.to_string().as_str()).map_err(FieldElementParseError::FromStrError)
        })
        .collect::<Result<Vec<Felt>, _>>()?;
    Ok((len, data))
}

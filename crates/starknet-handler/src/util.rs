use std::str::FromStr;

use primitive_types::U256;
use starknet::core::types::FieldElement;

use crate::error::{FieldElementParseError, HandlerError};

/// Splits a U256 state root into two u128 values representing the high and low parts.
///
/// # Arguments
///
/// * `state_root` - A string representing the state root.
///
/// # Returns
///
/// A tuple containing the high (u128) and low (u128) parts of the state root.
pub fn get_high_and_low(state_root: String) -> (u128, u128) {
    let state_root = U256::from_str(state_root.as_str()).unwrap();
    let state_root_low = state_root.low_u128();
    let state_root_high: u128 = (state_root >> 128).as_u128();
    (state_root_high, state_root_low)
}

/// Prepares an array of data for use in a StarkNet contract call.
///
/// # Arguments
///
/// * `data` - A vector of data elements that implement the `ToString` trait.
///
/// # Returns
///
/// A result containing a tuple with the length of the data (as a `FieldElement`) and the data itself (as a vector of `FieldElement`),
/// or a `HandlerError` if there is an error during conversion.
pub fn prepare_array_data<T: ToString>(
    data: Vec<T>,
) -> Result<(FieldElement, Vec<FieldElement>), HandlerError> {
    let len = FieldElement::from_dec_str(data.len().to_string().as_str()).unwrap();
    let data = data
        .iter()
        .map(|d| {
            FieldElement::from_dec_str(d.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)
        })
        .collect::<Result<Vec<FieldElement>, _>>()?;
    Ok((len, data))
}

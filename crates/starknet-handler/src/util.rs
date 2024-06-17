use std::str::FromStr;

use primitive_types::U256;

pub fn get_high_and_low(state_root: String) -> (u128, u128) {
    let state_root = U256::from_str(state_root.as_str()).unwrap();
    let state_root_low = state_root.low_u128();
    let state_root_high: u128 = (state_root >> 128).as_u128();
    (state_root_high, state_root_low)
}

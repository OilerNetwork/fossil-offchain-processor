use anyhow::Error;
use chrono::prelude::*;
use db_access::models::BlockHeader;
use std::collections::HashMap;

use super::utils::hex_string_to_f64;

#[derive(Debug, Clone)]
pub struct AggregatedBaseFee {
    pub base_fee_mean: f64,
    pub number: f64,
}

/// Calculates the time weighted average price (TWAP) of the base fee.
/// TODO: handle the unwraps properly, or at least propagate them upwards.
pub async fn calculate_twap(headers: Vec<BlockHeader>) -> Result<HashMap<String, f64>, Error> {
    let mut hourly_fee_data_mapping: HashMap<String, AggregatedBaseFee> = HashMap::new();

    headers.iter().for_each(|header| {
        let new_date = DateTime::<Utc>::from_timestamp(header.timestamp.unwrap(), 0).unwrap();
        let new_date_str = format!(
            "{}-{}-{} {}:00:00",
            new_date.year(),
            new_date.month(),
            new_date.day(),
            new_date.hour(),
        );

        if hourly_fee_data_mapping.contains_key(new_date_str.as_str()) {
            let current_data = hourly_fee_data_mapping
                .get_mut(new_date_str.as_str())
                .unwrap();
            current_data.base_fee_mean +=
                hex_string_to_f64(&header.base_fee_per_gas.clone().unwrap());
        } else {
            hourly_fee_data_mapping.insert(
                new_date_str.clone(),
                AggregatedBaseFee {
                    base_fee_mean: hex_string_to_f64(&header.base_fee_per_gas.clone().unwrap()),
                    number: 1f64,
                },
            );
        }
    });

    let hourly_fee_data_vec: Vec<(String, f64)> = hourly_fee_data_mapping
        .iter()
        .map(|(date, data)| (date.clone(), data.base_fee_mean / data.number))
        .collect();

    // Calculate the twap with a sliding window of 7 days.
    let mut twap_7d_mapping: HashMap<String, f64> = HashMap::new();
    hourly_fee_data_vec[..].windows(24 * 7).for_each(|window| {
        let twap_7d = window.iter().map(|(_, data)| data).sum::<f64>() / window.len() as f64;
        twap_7d_mapping.insert(window.first().unwrap().0.clone(), twap_7d);
    });

    Ok(twap_7d_mapping)
}

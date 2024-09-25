use anyhow::Error;
use chrono::prelude::*;
use db_access::queries::get_block_headers_by_time_range;
use db_access::DbConnection;
use std::collections::HashMap;

fn hex_to_i64(hex: String) -> i64 {
    i64::from_str_radix(hex.as_str().trim_start_matches("0x"), 16).unwrap()
}

#[derive(Debug, Clone)]
pub struct AggregatedFeeData {
    pub base_fee_mean: i64,
    pub gas_limit_mean: i64,
    pub gas_used_mean: i64,
    pub number: i64,
}

pub async fn calculate_twap(
    conn: &DbConnection,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Result<HashMap<String, i64>, Error> {
    let headers =
        get_block_headers_by_time_range(&conn.pool, start_timestamp, end_timestamp).await?;

    let mut hourly_fee_data_mapping: HashMap<String, AggregatedFeeData> = HashMap::new();

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
            current_data.base_fee_mean += hex_to_i64(header.base_fee_per_gas.clone().unwrap());
            current_data.gas_limit_mean += header.gas_limit.unwrap();
            current_data.gas_limit_mean += header.gas_limit.unwrap();
            current_data.gas_used_mean += header.gas_used.unwrap();
        } else {
            hourly_fee_data_mapping.insert(
                new_date_str.clone(),
                AggregatedFeeData {
                    base_fee_mean: hex_to_i64(header.base_fee_per_gas.clone().unwrap()),
                    gas_limit_mean: header.gas_limit.unwrap(),
                    gas_used_mean: header.gas_used.unwrap(),
                    number: 1,
                },
            );
        }
    });

    hourly_fee_data_mapping
        .iter_mut()
        .for_each(|(_date, data)| {
            data.base_fee_mean /= data.number;
            data.gas_limit_mean /= data.number;
            data.gas_used_mean /= data.number;
        });

    let hourly_fee_data_vec: Vec<(String, AggregatedFeeData)> = hourly_fee_data_mapping
        .iter()
        .map(|(date, data)| (date.clone(), data.clone()))
        .collect();

    // Calculate the twap with a sliding window of 7 days.
    let mut twap_7d_mapping: HashMap<String, i64> = HashMap::new();
    hourly_fee_data_vec[..].windows(24 * 7).for_each(|window| {
        let twap_7d: i64 = window
            .iter()
            .map(|(_, data)| data.base_fee_mean)
            .sum::<i64>()
            / window.len() as i64;
        twap_7d_mapping.insert(window.first().unwrap().0.clone(), twap_7d);
    });

    Ok(twap_7d_mapping)
}

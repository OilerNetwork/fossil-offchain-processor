use anyhow::{anyhow as err, Error};
use chrono::prelude::*;
use db_access::queries::get_block_headers_by_time_range;
use db_access::DbConnection;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct AggregatedFeeData {
    pub base_fee_mean: i64,
    pub gas_limit_mean: i64,
    pub gas_used_mean: i64,
    pub number: i64,
    pub twap_7d: Option<i64>,
}

pub async fn calculate_twap(
    conn: &DbConnection,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Result<HashMap<String, AggregatedFeeData>, Error> {
    let headers =
        get_block_headers_by_time_range(&conn.pool, start_timestamp, end_timestamp).await?;

    let mut hourly_fee_data_mapping: HashMap<String, AggregatedFeeData> = HashMap::new();

    headers.iter().for_each(|header| {
        let new_date =
            DateTime::<Utc>::from_timestamp(headers.last().unwrap().timestamp.unwrap(), 0).unwrap();
        let new_date_str = format!(
            "{}-{}-{}",
            new_date.year(),
            new_date.month(),
            new_date.day()
        );

        if hourly_fee_data_mapping.contains_key(new_date_str.as_str()) {
            let current_data = hourly_fee_data_mapping
                .get_mut(new_date_str.as_str())
                .unwrap();
            current_data.base_fee_mean +=
                i64::from_str(header.base_fee_per_gas.clone().unwrap().as_str()).unwrap();
            current_data.gas_limit_mean += header.gas_limit.unwrap();
            current_data.gas_limit_mean += header.gas_limit.unwrap();
            current_data.gas_used_mean += header.gas_used.unwrap();
        } else {
            hourly_fee_data_mapping.insert(
                new_date_str.clone(),
                AggregatedFeeData {
                    base_fee_mean: i64::from_str(header.base_fee_per_gas.clone().unwrap().as_str())
                        .unwrap(),
                    gas_limit_mean: header.gas_limit.unwrap(),
                    gas_used_mean: header.gas_used.unwrap(),
                    number: 1,
                    twap_7d: None,
                },
            );
        }
    });

    hourly_fee_data_mapping.iter_mut().for_each(|(date, data)| {
        data.base_fee_mean = data.base_fee_mean / data.number;
        data.gas_limit_mean = data.gas_limit_mean / data.number;
        data.gas_used_mean = data.gas_used_mean / data.number;
    });

    // Calculate the 7d twap
    let hourly_fee_data_vec: Vec<(String, AggregatedFeeData)> = hourly_fee_data_mapping
        .iter()
        .map(|(date, data)| (date.clone(), data.clone()))
        .collect();

    hourly_fee_data_vec[..].windows(24 * 7).for_each(|window| {
        let twap_7d: i64 = window
            .iter()
            .map(|(_, data)| data.base_fee_mean)
            .sum::<i64>()
            / window.len() as i64;
        window.iter().for_each(|(date, data)| {
            hourly_fee_data_mapping.get_mut(date).unwrap().twap_7d = Some(twap_7d);
        });
    });

    Ok(hourly_fee_data_mapping)
}

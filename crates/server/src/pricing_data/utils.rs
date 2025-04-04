use db_access::models::BlockHeader;
use eyre::{anyhow as err, Result};
use polars::prelude::*;

/// Converts a hex string to a f64 value.
///
/// # Arguments
///
/// * `hex_str` - The hex string to convert (can be prefixed with "0x" or not)
///
/// # Returns
///
/// A `Result` containing the converted f64 value, or an `Error` if the conversion fails.
///
/// # Errors
///
/// Returns an error if the hex string cannot be parsed as a u128.
pub fn hex_string_to_f64(hex_str: &String) -> Result<f64> {
    let stripped = hex_str.trim_start_matches("0x");
    u128::from_str_radix(stripped, 16)
        .map(|value| value as f64)
        .map_err(|e| eyre::eyre!("Error converting hex string '{}' to f64: {}", hex_str, e))
}

/// Loads block headers into a DataFrame with timestamp and base_fee fields.
///
/// # Arguments
///
/// * `block_headers` - A vector of `BlockHeader` structs containing the data to process
///
/// # Returns
///
/// A `Result` containing a `DataFrame` with timestamp and base_fee columns, or an `Error` if the operation fails.
///
/// # Errors
///
/// Returns an error if:
/// * No block headers are provided
/// * A header is missing a timestamp
/// * A header is missing a base fee
/// * The timestamp cannot be parsed as i64
/// * The base fee cannot be converted from hex to f64
pub fn prepare_data_frame(block_headers: Vec<BlockHeader>) -> Result<DataFrame> {
    if block_headers.is_empty() {
        tracing::error!("No block headers provided.");
        return Err(err!("No block headers provided."));
    }

    let mut timestamps = Vec::new();
    let mut base_fees = Vec::new();

    for header in block_headers {
        let timestamp = header
            .timestamp
            .ok_or_else(|| err!("No timestamp in header"))?
            .parse::<i64>()
            .map_err(|e| err!("Failed to parse timestamp as i64: {}", e))?;

        let base_fee = hex_string_to_f64(
            &header
                .base_fee_per_gas
                .ok_or_else(|| err!("No base fee in header"))?,
        )?;
        timestamps.push(timestamp);
        base_fees.push(base_fee);
    }

    let df = DataFrame::new(vec![
        Series::new("timestamp".into(), timestamps),
        Series::new("base_fee".into(), base_fees),
    ])?;

    Ok(df)
}

/// Replaces the timestamp column with a date column in a DataFrame.
///
/// This function takes a DataFrame with a timestamp column, converts the timestamps
/// to milliseconds, casts them to datetime, and replaces the timestamp column with
/// a new date column.
///
/// # Arguments
///
/// * `df` - The input DataFrame containing a timestamp column
///
/// # Returns
///
/// A `Result` containing the modified DataFrame with the timestamp column replaced
/// by the date column, or an `Error` if the operation fails.
///
/// # Errors
///
/// Returns an error if:
/// * The DataFrame is empty
/// * The timestamp column is missing or cannot be accessed
/// * The conversion to milliseconds or casting to datetime fails
/// * The column replacement or renaming operations fail
pub fn replace_timestamp_with_date(df: DataFrame) -> Result<DataFrame> {
    tracing::debug!("DataFrame shape before date conversion: {:?}", df.shape());

    if df.height() == 0 {
        return Err(err!(
            "Empty DataFrame provided to replace_timestamp_with_date"
        ));
    }

    tracing::debug!(
        "Time range: min={:?}, max={:?}",
        df.column("timestamp")?.min::<i64>(),
        df.column("timestamp")?.max::<i64>()
    );

    let timestamp_col = df.column("timestamp")?;

    let null_count = timestamp_col.null_count();
    if null_count > 0 {
        tracing::warn!("Found {} null values in timestamp column", null_count);
    }

    let dates = if timestamp_col.dtype() == &DataType::String {
        tracing::debug!("Converting string timestamps to integers");
        let int_timestamps = match timestamp_col.cast(&DataType::Int64) {
            Ok(ints) => ints.i64()?.apply(|s| s.map(|s| s * 1000)),
            Err(e) => {
                tracing::error!("Failed to cast string timestamps to integers: {:?}", e);
                return Err(err!("Failed to cast timestamps: {}", e));
            }
        };
        int_timestamps
            .into_series()
            .cast(&DataType::Datetime(TimeUnit::Milliseconds, None))?
            .rename("date".into())
            .clone()
    } else {
        tracing::debug!("Timestamp column is already numeric");
        timestamp_col
            .i64()?
            .apply(|s| s.map(|s| s * 1000))
            .into_series()
            .cast(&DataType::Datetime(TimeUnit::Milliseconds, None))?
            .rename("date".into())
            .clone()
    };

    let mut new_cols: Vec<Series> = Vec::new();
    for col in df.get_columns() {
        if col.name() != "timestamp" {
            new_cols.push(col.clone());
        }
    }
    new_cols.push(dates);

    let df = DataFrame::new(new_cols)?;
    tracing::debug!("DataFrame shape after date conversion: {:?}", df.shape());
    Ok(df)
}

/// Adds a Time-Weighted Average Price (TWAP) column to the DataFrame.
///
/// This function calculates the TWAP for the base_fee column and adds it as a new column
/// named TWAP_30d to the input DataFrame.
///
/// # Arguments
///
/// * `df` - The input DataFrame containing the base_fee column
/// * `window_size` - The size of the rolling window for the TWAP calculation
///   * If data is hourly grouped, 720 means 30d TWAP
///
/// # Returns
///
/// A `Result` containing the DataFrame with the added TWAP_30d column, or an `Error` if the operation fails.
///
/// # Errors
///
/// Returns an error if:
/// * There are insufficient rows for the TWAP calculation
/// * The rolling mean calculation fails
/// * The final collection of the lazy DataFrame fails
pub fn add_twaps(df: DataFrame, window_size: usize) -> Result<DataFrame> {
    if df.height() < window_size {
        return Err(err!(
            "Insufficient rows: need at least {} for TWAP, got {}.",
            window_size,
            df.height()
        ));
    }

    let df = df
        .lazy()
        .with_column(
            col("base_fee")
                .rolling_mean(RollingOptionsFixedWindow {
                    window_size,
                    min_periods: window_size,
                    weights: None,
                    center: false,
                    fn_params: None,
                })
                .alias("TWAP_30d"),
        )
        .collect()?;

    Ok(df)
}

/// Removes rows with null values in the specified column.
///
/// This function filters the input DataFrame to exclude null entries in the specified column.
///
/// # Arguments
///
/// * `df` - The input DataFrame to filter
/// * `column_name` - The name of the column to check for null values
///
/// # Returns
///
/// A `Result` containing the filtered DataFrame, or an `Error` if the operation fails.
pub fn drop_nulls(df: &DataFrame, column_name: &str) -> Result<DataFrame> {
    let df = df
        .clone()
        .lazy()
        .filter(col(column_name).is_not_null())
        .collect()?;

    Ok(df)
}

/// Groups a DataFrame into 1-hour or 1-minute basefee averages.
///
/// For data spanning over 7 days, the data is grouped into 1-hour averages.
/// For shorter (testnet) data ranges, the data is grouped by 1-minute averages.
///
/// # Arguments
///
/// * `df` - The input DataFrame to be grouped and aggregated
///
/// # Returns
///
/// A `Result` containing the grouped and aggregated DataFrame, or an `Error` if the operation fails.
///
/// # Errors
///
/// Returns an error if:
/// * The grouping or aggregation operations fail
/// * The final collection of the lazy DataFrame fails
pub fn group_by_1h_or_1m_intervals(df: DataFrame) -> Result<DataFrame> {
    tracing::debug!("DataFrame shape before grouping: {:?}", df.shape());
    // Calculate the total span in days
    let min_ts = df
        .column("date")?
        .datetime()?
        .min()
        .ok_or_else(|| err!("No min timestamp"))?;
    let max_ts = df
        .column("date")?
        .datetime()?
        .max()
        .ok_or_else(|| err!("No max timestamp"))?;

    let span_millis = max_ts - min_ts;
    let span_days = (span_millis as f64) / (1000.0 * 60.0 * 60.0 * 24.0);

    tracing::debug!("DataFrame length in days: {:?}", span_days);

    // Decide grouping interval (Tolerance to account for block gaps)
    let group_by = if span_days < 7.0 {
        tracing::info!(
            "Using 1-minute grouping (data span = {:.2} days)",
            span_days
        );
        "1m"
    } else {
        tracing::info!("Using 1-hour grouping (data span = {:.2} days)", span_days);
        "1h"
    };
    let (every, period) = (Duration::parse(group_by), Duration::parse(group_by));

    let df = match df
        .lazy()
        .group_by_dynamic(
            col("date"),
            [],
            DynamicGroupOptions {
                every,
                period,
                offset: Duration::parse("0"),
                ..Default::default()
            },
        )
        .agg([col("base_fee").mean()])
        .collect()
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Failed to group data by {:?} intervals: {:?}", group_by, e);
            return Err(err!("Failed to group data: {}", e));
        }
    };

    tracing::debug!("DataFrame shape after grouping: {:?}", df.shape());
    tracing::debug!("DataFrame: {:?}", df);

    Ok(df)
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;

    #[test]
    fn test_hex_string_to_f64_zero_value() {
        let result = hex_string_to_f64(&"0x0".to_string());

        assert_eq!(result.unwrap(), 0f64);
    }

    #[test]
    fn test_hex_string_to_f64_prefixed_value() {
        let result = hex_string_to_f64(&"0x12345".to_string());

        assert_eq!(result.unwrap(), 74565_f64);
    }

    #[test]
    fn test_hex_string_to_f64_non_prefixed_value() {
        let result = hex_string_to_f64(&"12345".to_string());

        assert_eq!(result.unwrap(), 74565_f64);
    }

    #[test]
    fn test_hex_string_to_f64_invalid_value() {
        let result = hex_string_to_f64(&"shouldpanic".to_string());

        assert!(result.is_err(), "Expected an error, but got {:?}", result);
    }

    // Returns a Vec of timestamps
    //
    // # Arguments
    //
    // * `start` - The starting unix timestamp value (e.g: 1743448159)
    // * `amount` - The number of timestamps to generate.
    // * `step` - The interval between timestamps in seconds (e.g: 12)
    fn create_timestamps(start: i64, amount: usize, step: i64) -> Vec<String> {
        let mut timestamps: Vec<String> = Vec::new();
        for i in 0..amount {
            let i: i64 = i.try_into().expect("Failed to convert i to i64");
            let timestamp = start + i * step;
            timestamps.push(timestamp.to_string());
        }

        timestamps
    }

    // Returns a Vec of base fees in the specified range
    //
    // # Arguments
    //
    // * `amount` - The number of base_fees to generate.
    // * `range` - The range for random base fee values.
    fn create_base_fees(amount: usize, range: (f64, f64)) -> Vec<f64> {
        let mut base_fees: Vec<f64> = Vec::new();
        for _ in 0..amount {
            let base_fee = rand::thread_rng().gen_range(range.0..=range.1);
            base_fees.push(base_fee);
        }

        base_fees
    }

    #[test]
    fn test_create_timestamps() {
        let amount = 5;
        let result = create_timestamps(1, amount, 12);

        // Check size
        assert_eq!(result.len(), amount);

        // Check all values are strings
        assert_eq!(result[0], "1");
        assert_eq!(result[1], "13");
        assert_eq!(result[2], "25");
        assert_eq!(result[3], "37");
        assert_eq!(result[4], "49");
    }

    #[test]
    fn test_create_base_fees() {
        let amount = 5;
        let range = (4000000000.0, 5000000000.0);
        let result = create_base_fees(amount, range);

        // Check size
        assert_eq!(result.len(), amount);

        // Check all are in range
        for base_fee in &result {
            let value = base_fee.clone();
            assert!(value >= range.0 && value <= range.1);
        }
    }

    #[test]
    fn test_group_by_1m_intervals() {
        let midnight_jan_1 = 1735707600;

        let step = 12;
        let amount = (3600 * 3) / step; // 3 hours
        let timestamps = create_timestamps(midnight_jan_1, amount, step as i64);
        let base_fees = create_base_fees(amount, (4_000_000_000.0, 5_000_000_000.0));

        let df: DataFrame = DataFrame::new(vec![
            Series::new("timestamp".into(), timestamps),
            Series::new("base_fee".into(), base_fees),
        ])
        .expect("Failed to create DataFrame");
        let df = replace_timestamp_with_date(df).expect("Failed to convert timestamps");
        let df = group_by_1h_or_1m_intervals(df).expect("Failed to group");

        // Check shape
        let in_minutes = (amount * step as usize) / 60;
        assert_eq!(df.shape(), (in_minutes, 2));
    }

    #[test]
    fn test_group_by_1h_intervals() {
        let midnight_jan_1 = 1735707600;

        let step: usize = 12;
        let amount: usize = (24 * 3600 * 30 * 5) / step; // 150 days
        let timestamps = create_timestamps(midnight_jan_1, amount, step as i64);
        let base_fees = create_base_fees(amount, (4_000_000_000.0, 5_000_000_000.0));

        let df: DataFrame = DataFrame::new(vec![
            Series::new("timestamp".into(), timestamps),
            Series::new("base_fee".into(), base_fees),
        ])
        .expect("Failed to create DataFrame");

        let df = replace_timestamp_with_date(df).expect("Failed to convert timestamps");
        let df = group_by_1h_or_1m_intervals(df).expect("Failed to group");

        // Check shape
        let in_hours = 24 * 30 * 5;
        assert_eq!(df.shape(), (in_hours, 2));
    }
}

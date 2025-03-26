use db_access::models::BlockHeader;
use eyre::{anyhow as err, Result};
use polars::prelude::*;

use super::utils::hex_string_to_f64;

/// Calculates the volatility of the returns over the final window of the data.
/// Mainnet/ZKVM will use hardcoded 30d (TWAP/returns) and 90d (volatility) values;
/// however, testnet will by dynamic, allowing for shorter vaults to use this service
///  1. Derive (twap_window, vol_window) from total timespan
///    - For a 3 hour vault, we will pass 5 * 3 = 15 hours of block headers
///    - This means we will use 15 * (1/5) = 3 hour TWAPs to calculate 3 hour returns
///    - Then we will find the volatility of returns over the final 15 * (3/5) = 9 hours
pub async fn calculate_volatility(block_headers: Vec<BlockHeader>) -> Result<f64> {
    if block_headers.is_empty() {
        return Err(err!("No block headers provided."));
    }

    // 1. Prepare DataFrame
    let mut timestamps = Vec::new();
    let mut base_fees = Vec::new();

    for header in block_headers {
        let timestamp = header
            .timestamp
            .ok_or_else(|| err!("No timestamp in header"))?;
        let base_fee = hex_string_to_f64(
            &header
                .base_fee_per_gas
                .ok_or_else(|| err!("No base fee in header"))?,
        )?;
        timestamps.push(timestamp);
        base_fees.push(base_fee);
    }

    let mut df = DataFrame::new(vec![
        Series::new("timestamp".into(), timestamps.clone()),
        Series::new("base_fee".into(), base_fees),
    ])?;

    // 2. Convert timestamps to dates & group by 1h avgs
    df = replace_timestamp_with_date(df)?;
    df = group_by_1h_intervals(df)?;

    // 3. Compute dynamic windows
    let (twap_window, vol_window) = compute_twap_and_vol_windows(&timestamps);
    if twap_window == 0 || vol_window == 0 {
        return Err(err!("Not enough timestamps or invalid time range."));
    }

    tracing::info!(
        "Using twap_window={} hours, vol_window={} hours",
        twap_window,
        vol_window
    );

    // 4. TWAP & returns & volatility
    df = calculate_twaps(df, twap_window)?;
    df = drop_nulls(&df, "TWAP_X")?;

    df = calculate_returns(df, twap_window)?;
    df = drop_nulls(&df, "X_returns")?;

    df = compute_volatilitys(df, vol_window)?;
    df = drop_nulls(&df, "volatility_X")?;

    // 5. Get the final volatility value by calculating the standard deviation of the final vol_window chunk
    let max_date = df
        .column("date")?
        .datetime()?
        .max()
        .ok_or_else(|| err!("No date values"))?;
    let min_cutoff = max_date - (vol_window as i64 * 3600_000); // convert hours to milliseconds

    // Lazy filter for date >= min_cutoff
    let lazy_df = df
        .lazy()
        .filter(col("date").gt(lit(min_cutoff)))
        .collect()?;

    if lazy_df.height() == 0 {
        return Err(err!(
            "No rows left after filtering to final volatility window."
        ));
    }

    // Compute std dev of "30d_returns" in the final chunk (volatility)
    let volatility = lazy_df
        .column("X_returns")?
        .f64()?
        .std(1)
        .ok_or_else(|| eyre::eyre!("No data to compute volatility"))?;

    Ok(10_000.0 * volatility)
}

/// Replaces the 'timestamp' column with a 'date' column in a DataFrame.
fn replace_timestamp_with_date(df: DataFrame) -> Result<DataFrame> {
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

/// Groups the DataFrame by 1-hour intervals and aggregates specified columns.
///
/// This function takes a DataFrame and groups it by 1-hour intervals based on the 'date' column.
/// It then calculates the mean values for 'base_fee' within each interval.
fn group_by_1h_intervals(df: DataFrame) -> Result<DataFrame> {
    tracing::debug!("DataFrame shape before grouping: {:?}", df.shape());

    // Add a warning for very large datasets
    if df.height() > 10000 {
        tracing::warn!(
            "Processing a large dataset with {} rows. This may take some time.",
            df.height()
        );
    }

    let df = match df
        .lazy()
        .group_by_dynamic(
            col("date"),
            [],
            DynamicGroupOptions {
                every: Duration::parse("1h"),
                period: Duration::parse("1h"),
                offset: Duration::parse("0"),
                ..Default::default()
            },
        )
        .agg([col("base_fee").mean()])
        .collect()
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Failed to group data by 1h intervals: {:?}", e);
            return Err(err!("Failed to group data: {}", e));
        }
    };

    tracing::debug!("DataFrame shape after grouping: {:?}", df.shape());
    tracing::debug!("DataFrame: {:?}", df);

    Ok(df)
}

// Removes rows with null values in the specified column and returns a new DataFrame
fn drop_nulls(df: &DataFrame, column_name: &str) -> Result<DataFrame> {
    let df = df
        .clone()
        .lazy()
        .filter(col(column_name).is_not_null())
        .collect()?;

    Ok(df)
}

/// Divides total data range into 5 parts:
///   1 part  -> TWAP window
///   3 parts -> final "vol window"
///   1 part  -> leftover warm-up
/// Returns (twap_window_hours, vol_window_hours).
fn compute_twap_and_vol_windows(hex_timestamps: &[String]) -> (usize, usize) {
    let parsed: Vec<i64> = hex_timestamps
        .iter()
        .filter_map(|s| i64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .collect();

    if parsed.len() < 2 {
        return (0, 0);
    }

    let min_ts = *parsed.iter().min().unwrap();
    let max_ts = *parsed.iter().max().unwrap();
    let span_secs = max_ts - min_ts;
    if span_secs <= 0 {
        return (0, 0);
    }

    let span_hours = (span_secs as f64) / 3600.0;

    // 1 chunk = TWAP window, 3 chunks = vol window, 1 leftover chunk
    let twap_f = span_hours / 5.0; // "30d" portion
    let vol_f = twap_f * 3.0; // "90d" portion in that ratio

    let twap_window = twap_f.floor() as usize;
    let vol_window = vol_f.floor() as usize;

    // At most 30 days for each window
    let twap_window = twap_window.min(24 * 30);
    let vol_window = vol_window.min(24 * 30 * 3);

    (twap_window, vol_window)
}

// Calculates the Time-Weighted Average Price (TWAP) over a specified window size.
fn calculate_twaps(df: DataFrame, window_size: usize) -> Result<DataFrame> {
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
                    min_periods: 1,
                    ..Default::default()
                })
                .alias("TWAP_X"),
        )
        .collect()?;

    Ok(df.fill_null(FillNullStrategy::Backward(None))?)
}

/// Calculates 30-day returns based on TWAP values (column "TWAP_30d").
fn calculate_returns(df: DataFrame, period: usize) -> Result<DataFrame> {
    let df = df
        .lazy()
        .with_column(
            (col("TWAP_X") / col("TWAP_X").shift(lit(period as i64)) - lit(1.0)).alias("X_returns"),
        )
        .collect()?;

    Ok(df)
}

// (NOT NEEDED)
// Rolling volatility over 'window_size' rows on "X_returns" column.
// By default, Polars uses `ddof=1` for rolling_std if you don't specify it.
// That corresponds to the sample standard deviation.
fn compute_volatilitys(df: DataFrame, window_size: usize) -> Result<DataFrame> {
    if df.height() < window_size {
        return Err(err!(
            "Insufficient rows: need at least {} for volatility, got {}.",
            window_size,
            df.height()
        ));
    }

    // Use lazy API to add a new column "VOL_X" containing the rolling std dev of "returns_X".
    let df = df
        .lazy()
        .with_column(
            col("X_returns")
                .rolling_std(RollingOptionsFixedWindow {
                    window_size,
                    min_periods: 1,
                    // If you want to customize ddof, you can set it explicitly:
                    // ddof: 1,
                    ..Default::default()
                })
                .alias("volatility_X"),
        )
        .collect()?;

    // Fill nulls if needed.
    Ok(df.fill_null(FillNullStrategy::Backward(None))?)
}

use db_access::models::BlockHeader;
use eyre::{anyhow as err, Result};
use polars::prelude::*;

use super::utils::hex_string_to_f64;

/// Calculate cap level using volatility, alpha, and k
/// @param volatility: volatility of returns as a decimal (e.g., 0.33 is 33%)
/// @param alpha: target percentage of max returns in BPS (e.g., 5000 for 50%)
/// @param k: strike level in BPS (e.g., -2500 for -25%)
///
/// cl = (λ - k) / (α * (1 + k))
/// - λ = 2.33 x volatility: 0% <= λ < ∞%
/// - k: -100.00% < k < ∞%
/// - a: 0.00% < a <= 100%
pub async fn calculate_cap_level(alpha: u128, k: i128, blocks: Vec<BlockHeader>) -> Result<f64> {
    // Calculate volatility
    let volatility = calculate_volatility(blocks).await?;
    tracing::info!("Calculate volatiltiy: {}", volatility);

    // Get percentage values for each variable
    let lambda = 2.33 * volatility;
    let alpha = (alpha as f64) / 10_000.0;
    let k = (k as f64) / 10_000.0;

    let cap_level = (lambda - k) / (alpha * (1.0 + k));

    Ok(cap_level)
}

/// Calculates the volatility of returns over a window.
///
/// Mainnet/zkvm (30d vaults): 1: Calculate 30d TWAPs. 2: Calculate 30d returns. 3: Calculate 90d volatility.
/// Testnet (any length vaults): First, calculate TWAP/return/volatility window for dynamic vaults, then calculate volatility.
///
/// - For a 3 hour vault, we will pass 5 * 3 = 15 hours of block headers
/// - TWAP & return window: 15 * (1/5) = 3 hours
/// - Volatility window: 15 * (3/5) = 9 hours
pub async fn calculate_volatility(block_headers: Vec<BlockHeader>) -> Result<f64> {
    if block_headers.is_empty() {
        return Err(err!("No block headers provided."));
    }

    // Prepare DataFrame
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

    // Convert timestamps to dates
    df = replace_timestamp_with_date(df)?;

    // Group by 1-hour intervals for 30d vaults (or by 1-minute for < 30d vaults)
    df = group_by_1h_or_1m_intervals(df)?;

    // TWAP window is 20% of the data size (30d max)
    let twap_window = ((df.height() as f64) * 0.2).floor().clamp(1.0, 24.0 * 30.0) as usize;

    // Volatility window is 3x the TWAP window (90d max)
    let vol_window = 3 * twap_window;

    tracing::info!(
        "Using twap_window={} hours, vol_window={} hours",
        twap_window,
        vol_window
    );

    // 1. Calculate rolling TWAPs
    df = calculate_twaps(df, twap_window)?;
    df = drop_nulls(&df, "TWAP_X")?;

    // 2. Calculate rolling returns
    df = calculate_returns(df, twap_window)?;
    df = drop_nulls(&df, "X_returns")?;

    // 3. Calculate volatility over final `vol_window` rows (standard deviation of returns)
    let start_idx = df.height().saturating_sub(vol_window);
    let final_chunk = df.slice(start_idx as i64, vol_window);

    if final_chunk.height() == 0 {
        return Err(err!(
            "No rows left after slicing to final volatility window."
        ));
    }

    // Compute standard deviation of returns in that final chunk
    let volatility = final_chunk
        .column("X_returns")?
        .f64()?
        .std(1) // sample std dev
        .ok_or_else(|| eyre::eyre!("No data to compute volatility"))?;

    Ok(volatility)
    // (NOT NEEDED)
    // Compute rolling volatility
    //df = _compute_volatilitys(df, vol_window)?;
    //df = drop_nulls(&df, "volatility_X")?;
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

/// Groups the DataFrame by 1-hour intervals for 30d vaults and aggregates specified columns.
/// - For shorter vaults, the data is grouped by 1-minute intervals.
///
/// It then calculates the mean values for 'base_fee' within each interval.
fn group_by_1h_or_1m_intervals(df: DataFrame) -> Result<DataFrame> {
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
    let group_by = if span_days < 149.0 {
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

// Removes rows with null values in the specified column and returns a new DataFrame
fn drop_nulls(df: &DataFrame, column_name: &str) -> Result<DataFrame> {
    let df = df
        .clone()
        .lazy()
        .filter(col(column_name).is_not_null())
        .collect()?;

    Ok(df)
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
fn _compute_volatilitys(df: DataFrame, window_size: usize) -> Result<DataFrame> {
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

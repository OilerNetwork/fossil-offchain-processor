use db_access::models::BlockHeader;
use eyre::{anyhow as err, Result};
use polars::prelude::*;

use super::utils::hex_string_to_f64;

pub async fn calculate_max_returns(block_headers: Vec<BlockHeader>) -> Result<f64> {
    if block_headers.is_empty() {
        tracing::error!("No block headers provided.");
        return Err(eyre::eyre!("No block headers provided."));
    }

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
        Series::new("timestamp".into(), timestamps),
        Series::new("base_fee".into(), base_fees),
    ])?;

    df = replace_timestamp_with_date(df)?;
    df = group_by_1h_intervals(df)?;

    df = add_twap_30d(df)?;
    df = drop_nulls(&df, "TWAP_30d")?;
    df = calculate_30d_returns(df)?;
    df = drop_nulls(&df, "30d_returns")?;

    let max_return = df.column("30d_returns")?.f64()?.max().ok_or_else(|| {
        tracing::error!("30d returns series is empty.");
        err!("30d returns series is empty")
    })?;

    Ok(max_return)
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

/// Adds a Time-Weighted Average Price (TWAP) column to the DataFrame.
///
/// This function calculates the 30-day TWAP for the 'base_fee' column and adds it as a new column
/// named 'TWAP_30d' to the input DataFrame.
///
/// # Arguments
///
/// * `df` - The input DataFrame containing the 'base_fee' column.
///
/// # Returns
///
/// A `Result` containing the DataFrame with the added 'TWAP_30d' column, or an `Error` if the
/// operation fails.
///
/// # Errors
///
/// This function will return an error if:
/// * The rolling mean calculation fails.
/// * The final collection of the lazy DataFrame fails.
///
fn add_twap_30d(df: DataFrame) -> Result<DataFrame> {
    let required_window_size = 24 * 30;

    tracing::debug!("DataFrame shape before TWAP: {:?}", df.shape());

    if df.height() < required_window_size {
        return Err(err!(
            "Insufficient data: At least {} data points are required, but only {} provided.",
            required_window_size,
            df.height()
        ));
    }

    let lazy_df = df.lazy().with_column(
        col("base_fee")
            .rolling_mean(RollingOptionsFixedWindow {
                window_size: required_window_size,
                min_periods: 1,
                weights: None,
                center: false,
                fn_params: None,
            })
            .alias("TWAP_30d"),
    );

    let df = lazy_df.collect()?;
    tracing::debug!("DataFrame shape after TWAP: {:?}", df.shape());

    Ok(df.fill_null(FillNullStrategy::Backward(None))?)
}

/// Groups the DataFrame by 1-hour intervals and aggregates specified columns.
///
/// This function takes a DataFrame and groups it by 1-hour intervals based on the 'date' column.
/// It then calculates the mean values for 'base_fee' within each interval.
///
/// # Arguments
///
/// * `df` - The input DataFrame to be grouped and aggregated.
///
/// # Returns
///
/// A `Result` containing the grouped and aggregated DataFrame, or an `Error` if the operation fails.
///
/// # Errors
///
/// This function will return an error if:
/// * The grouping or aggregation operations fail.
/// * The final collection of the lazy DataFrame fails.
///
fn group_by_1h_intervals(df: DataFrame) -> Result<DataFrame> {
    tracing::debug!("DataFrame shape before grouping: {:?}", df.shape());

    let df = df
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
        .collect()?;

    tracing::debug!("DataFrame shape after grouping: {:?}", df.shape());
    tracing::debug!("DataFrame: {:?}", df);

    Ok(df)
}

/// Replaces the 'timestamp' column with a 'date' column in a DataFrame.
///
/// This function takes a DataFrame with a 'timestamp' column, converts the timestamps
/// to milliseconds, casts them to datetime, and replaces the 'timestamp' column with
/// a new 'date' column.
///
/// # Arguments
///
/// * `df` - A mutable reference to the input DataFrame.
///
/// # Returns
///
/// A `Result` containing the modified DataFrame with the 'timestamp' column replaced
/// by the 'date' column, or an `Error` if the operation fails.
///
/// # Errors
///
/// This function will return an error if:
/// * The 'timestamp' column is missing or cannot be accessed.
/// * The conversion to milliseconds or casting to datetime fails.
/// * The column replacement or renaming operations fail.
///
fn replace_timestamp_with_date(df: DataFrame) -> Result<DataFrame> {
    tracing::debug!("DataFrame shape before date conversion: {:?}", df.shape());
    tracing::debug!(
        "Time range: min={:?}, max={:?}",
        df.column("timestamp")?.i64()?.min(),
        df.column("timestamp")?.i64()?.max()
    );

    let dates = df
        .column("timestamp")?
        .i64()?
        .apply(|s| s.map(|s| s * 1000))
        .into_series()
        .cast(&DataType::Datetime(TimeUnit::Milliseconds, None))?
        .rename("date".into())
        .clone();

    // Create a new DataFrame with the date column instead of timestamp
    let mut new_cols: Vec<Series> = Vec::new();
    for col in df.get_columns() {
        if col.name() != "timestamp" {
            new_cols.push(col.clone());
        }
    }
    new_cols.push(dates);

    let df = DataFrame::new(new_cols)?;

    tracing::debug!("DataFrame shape after date conversion: {:?}", df.shape());
    tracing::debug!(
        "DataFrame columns after conversion: {:?}",
        df.get_column_names()
    );

    Ok(df)
}

/// Calculates 30-day returns based on TWAP values.
///
/// This function computes the 30-day returns by dividing each TWAP value by its value
/// from 30 days ago (720 hours) and subtracting 1 to get the percentage return.
/// The result is added as a new column '30d_returns'.
///
/// For example, if TWAP_30d(t) = 150 and TWAP_30d(t-30d) = 100,
/// then 30d_returns(t) = 150/100 - 1 = 0.5 (50% return)
///
/// # Arguments
///
/// * `df` - The input DataFrame containing the 'TWAP_30d' column
///
/// # Returns
///
/// A `Result` containing the DataFrame with the added '30d_returns' column and nulls dropped,
/// or an `Error` if the operation fails.
///
/// # Errors
///
/// This function will return an error if:
/// * The 'TWAP_30d' column is missing or cannot be accessed
/// * The shift operation fails
/// * The division operation fails
/// * The final collection of the lazy DataFrame fails
fn calculate_30d_returns(df: DataFrame) -> Result<DataFrame> {
    tracing::debug!(
        "DataFrame shape before calculating returns: {:?}",
        df.shape()
    );

    // 24 hours * 30 days = 720 hours
    let period = 24 * 30;

    let df = df
        .lazy()
        .with_column(
            (col("TWAP_30d") / col("TWAP_30d").shift(lit(period)) - lit(1.0)).alias("30d_returns"),
        )
        .collect()?;

    tracing::debug!(
        "DataFrame shape after calculating returns: {:?}",
        df.shape()
    );
    Ok(df)
}

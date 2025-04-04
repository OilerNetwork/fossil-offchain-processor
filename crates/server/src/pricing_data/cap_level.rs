use db_access::models::BlockHeader;
use eyre::{anyhow as err, Result};
use polars::prelude::*;

use super::utils::{
    add_twaps, drop_nulls, group_by_1h_or_1m_intervals, prepare_data_frame,
    replace_timestamp_with_date,
};

/// Calculate cap level to use for the upcoming round
///
/// @param alpha: target percentage of max returns in BPS (e.g., 5000 for 50%)
/// @param k: strike level in BPS (e.g., -2500 for -25%)
/// @param blocks: list of block headers
/// - Requires `5 * 30d = 150d` of block headers for zkvm/mainnet (testnet uses shorter vaults. i.e 5 * 12m = 1h of block headers)
///
/// cl = (λ - k) / (α * (1 + k)): 0% <= cl < ∞%
/// - λ = 2.33 x volatility: 0% <= λ < ∞%
/// - k: -100.00% < k < ∞%
/// - a: 0.00% < a <= 100%
pub async fn calculate_cap_level(alpha: u128, k: i128, blocks: Vec<BlockHeader>) -> Result<f64> {
    // Validate alpha and k bounds
    if alpha > 10_000 || alpha <= 0 {
        return Err(err!("Invalid alpha value: {}", alpha));
    }
    if k <= -10000 {
        return Err(err!("Invalid k value: {}", k));
    }

    // Calculate volatility
    let volatility = calculate_volatility(blocks).await?;
    tracing::info!("Calculated volatiltiy: {}", volatility);

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
    // Prepare data frame
    let mut df = prepare_data_frame(block_headers)?;

    // Convert timestamps to dates
    df = replace_timestamp_with_date(df)?;

    // Group by 1-hour intervals for 30d vaults (or by 1-minute for < 30d vaults)
    df = group_by_1h_or_1m_intervals(df)?;

    // For 30d vaults (zkvm/mainnet), twap_window is `720` (30d in hours)
    // For testnet, twap_window is 20% of the data size
    // - if a 12 min vault passes 5 * 12 = 60min (1h) of block headers
    // - TWAP window: 60 * (1/5) = 12min
    let twap_window = ((df.height() as f64) * 0.2).floor().clamp(1.0, 720.0) as usize;

    // For 30d vaults, vol_window is `2160` (90d in hours)
    let vol_window = 3 * twap_window;

    tracing::info!(
        "Using twap_window={} hours, vol_window={} hours",
        twap_window,
        vol_window
    );

    // 1. Calculate rolling TWAPs
    df = add_twaps(df, twap_window)?;
    df = drop_nulls(&df, "TWAP_30d")?;

    // 2. Calculate rolling returns
    df = calculate_returns(df, twap_window)?;
    df = drop_nulls(&df, "30d_returns")?;

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
        .column("30d_returns")?
        .f64()?
        .std(1) // sample std dev
        .ok_or_else(|| eyre::eyre!("No data to compute volatility"))?;

    Ok(volatility)

    // (NOT NEEDED)
    // Compute rolling volatility
    //df = _compute_volatilitys(df, vol_window)?;
    //df = drop_nulls(&df, "volatility_X")?;
}

/// Calculates 30-day returns based on TWAP values (column "TWAP_30d").
fn calculate_returns(df: DataFrame, period: usize) -> Result<DataFrame> {
    let df = df
        .lazy()
        .with_column(
            (col("TWAP_30d") / col("TWAP_30d").shift(lit(period as i64)) - lit(1.0))
                .alias("30d_returns"),
        )
        .collect()?;

    Ok(df)
}

// (NOT NEEDED)
// Rolling volatility over 'window_size' rows on "30d_returns" column.
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
            col("30d_returns")
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

use db_access::models::EthBlockHeader;
use ndarray_linalg::LeastSquaresSvd;

use super::utils::hex_string_to_f64;
use chrono::prelude::*;
use eyre::{anyhow as err, Result};
use linfa::prelude::*;
use linfa::traits::Fit;
use linfa_linear::{FittedLinearRegression, LinearRegression};
use ndarray::prelude::*;
use ndarray::{stack, Array1, Array2, Axis};
use ndarray_rand::rand_distr::Normal;
use optimization::{Func, GradientDescent, Minimizer, NumericalDifferentiation};
use polars::prelude::*;
use rand::prelude::*;
use rand_distr::Distribution;
use statrs::distribution::Binomial;
use std::f64::consts::PI;

pub async fn calculate_reserve_price(block_headers: Vec<EthBlockHeader>) -> Result<f64> {
    tracing::info!("Starting reserve price calculation.");
    if block_headers.is_empty() {
        tracing::error!("No block headers provided.");
        return Err(eyre::eyre!("No block headers provided."));
    }

    let mut timestamps = Vec::new();
    let mut base_fees = Vec::new();

    for header in block_headers {
        tracing::debug!("Processing header: {:?}", header);
        let timestamp = header
            .timestamp
            .ok_or_else(|| err!("No timestamp in header"))?;
        let base_fee = hex_string_to_f64(
            &header
                .base_fee_per_gas
                .ok_or_else(|| err!("No base fee in header"))?,
        )?;
        tracing::debug!(
            "Parsed timestamp: {} | Parsed base fee: {}",
            timestamp,
            base_fee
        );

        timestamps.push(timestamp);
        base_fees.push(base_fee);
    }

    tracing::debug!("Collected timestamps: {:?}", timestamps);
    tracing::debug!("Collected base fees: {:?}", base_fees);

    let mut df = DataFrame::new(vec![
        Series::new("timestamp".into(), timestamps),
        Series::new("base_fee".into(), base_fees),
    ])?;

    tracing::debug!("Initial DataFrame: {:?}", df);

    df = replace_timestamp_with_date(df)?;
    tracing::debug!("After date conversion: {:?}", df);

    df = group_by_1h_intervals(df)?;
    tracing::debug!("After grouping by 1h intervals: {:?}", df);

    df = add_twap_7d(df)?;
    let twap_7d_series = df.column("TWAP_7d")?;
    let strike = twap_7d_series.f64()?.last().ok_or_else(|| {
        tracing::error!("The series is empty.");
        err!("The series is empty")
    })?;

    tracing::info!("Strike price: {}", strike);

    let num_paths = 15000;
    let n_periods = 720;
    let cap_level = 0.3;
    let risk_free_rate = 0.05;

    let mut df = drop_nulls(&df, "TWAP_7d")?;
    tracing::debug!("After dropping nulls: {:?}", df);

    let period_end_date_timestamp = df
        .column("date")?
        .datetime()?
        .get(df.height() - 1)
        .ok_or_else(|| err!("No row {} in the date column", df.height() - 1))?;
    tracing::debug!("Period end timestamp: {}", period_end_date_timestamp);

    let period_start_date_timestamp = df
        .column("date")?
        .datetime()?
        .get(0)
        .ok_or_else(|| err!("No row 0 in the date column"))?;
    tracing::debug!("Period start timestamp: {}", period_start_date_timestamp);

    let log_base_fee = compute_log_of_base_fees(&df)?;
    tracing::debug!("Log base fees: {:?}", log_base_fee);

    df.with_column(Series::new("log_base_fee".into(), log_base_fee))?;

    let (trend_model, trend_values) = discover_trend(&df)?;
    tracing::debug!("Trend model params: {:?}", trend_model.params());
    tracing::debug!("Trend values: {:?}", trend_values);

    df.with_column(Series::new("trend".into(), trend_values))?;
    df.with_column(Series::new(
        "detrended_log_base_fee".into(),
        df["log_base_fee"].f64()? - df["trend"].f64()?,
    ))?;

    let (de_seasonalised_detrended_log_base_fee, season_param) =
        remove_seasonality(&mut df, period_start_date_timestamp)?;
    df.with_column(Series::new(
        "de_seasonalized_detrended_log_base_fee".into(),
        de_seasonalised_detrended_log_base_fee.clone().to_vec(),
    ))?;

    let (de_seasonalized_detrended_simulated_prices, _params) = simulate_prices(
        de_seasonalised_detrended_log_base_fee.view(),
        n_periods,
        num_paths,
    )?;

    tracing::debug!(
        "Simulated detrended prices: {:?}",
        de_seasonalized_detrended_simulated_prices
    );

    let total_hours = (period_end_date_timestamp - period_start_date_timestamp) / 3600 / 1000;
    tracing::debug!("Total hours in period: {}", total_hours);

    let sim_hourly_times: Array1<f64> =
        Array1::range(0.0, n_periods as f64, 1.0).mapv(|i| total_hours as f64 + i);

    let c = season_matrix(sim_hourly_times);
    let season = c.dot(&season_param);
    tracing::debug!("Season matrix product: {:?}", season);

    let season_reshaped = season.into_shape((n_periods, 1)).unwrap();

    let detrended_simulated_prices = &de_seasonalized_detrended_simulated_prices + &season_reshaped;

    let log_twap_7d: Vec<f64> = df
        .column("TWAP_7d")?
        .f64()?
        .into_no_null_iter()
        .map(|x| x.ln())
        .collect();
    tracing::debug!("Log TWAP 7d: {:?}", log_twap_7d);

    let returns: Vec<f64> = log_twap_7d
        .windows(2)
        .map(|window| window[1] - window[0])
        .collect();
    let returns: Vec<f64> = returns.into_iter().filter(|&x| !x.is_nan()).collect();
    tracing::debug!("Returns: {:?}", returns);

    let mu = 0.05 / 52.0;
    let sigma = standard_deviation(returns) * f64::sqrt(24.0 * 7.0);
    tracing::debug!("Sigma: {}", sigma);

    let dt = 1.0 / 24.0;

    let mut stochastic_trend = Array2::<f64>::zeros((n_periods, num_paths));
    let normal = Normal::new(0.0, sigma * (f64::sqrt(dt))).unwrap();
    let mut rng = thread_rng();
    for i in 0..num_paths {
        let random_shocks: Vec<f64> = (0..n_periods).map(|_| normal.sample(&mut rng)).collect();
        let mut cumsum = 0.0;
        for j in 0..n_periods {
            cumsum += (mu - 0.5 * sigma.powi(2)) * dt + random_shocks[j];
            stochastic_trend[[j, i]] = cumsum;
        }
    }

    tracing::debug!("Stochastic trend: {:?}", stochastic_trend);

    let coeffs = trend_model.params();
    let final_trend_value = {
        let x = (df.height() - 1) as f64;
        coeffs[0] * x + coeffs[1]
    };

    let mut simulated_log_prices = Array2::<f64>::zeros((n_periods, num_paths));
    for i in 0..n_periods {
        let trend = final_trend_value;
        for j in 0..num_paths {
            simulated_log_prices[[i, j]] =
                detrended_simulated_prices[[i, j]] + trend + stochastic_trend[[i, j]];
        }
    }

    let simulated_prices = simulated_log_prices.mapv(f64::exp);
    let twap_start = n_periods.saturating_sub(24 * 7);
    let final_prices_twap = simulated_prices
        .slice(s![twap_start.., ..])
        .mean_axis(Axis(0))
        .unwrap();

    let payoffs = final_prices_twap.mapv(|price| {
        let capped_price = (1.0 + cap_level) * strike;
        (price.min(capped_price) - strike).max(0.0)
    });

    let average_payoff = payoffs.mean().unwrap_or(0.0);
    let reserve_price = f64::exp(-risk_free_rate) * average_payoff;

    Ok(reserve_price)
}

/// Removes seasonality from the detrended log base fee and adds relevant columns to the DataFrame.
///
/// This function creates a time series, computes the seasonal component, and removes it from the
/// detrended log base fee. It adds new columns to the DataFrame for the time series and the
/// de-seasonalized detrended log base fee.
///
/// # Arguments
///
/// * `df` - A mutable reference to the DataFrame containing the data.
/// * `start_date_timestamp` - The timestamp of the start date.
///
/// # Returns
///
/// A Result containing a tuple with two elements:
/// * The de-seasonalized detrended log base fee as an Array1<f64>
/// * The seasonal parameters as an Array1<f64>
///
/// Returns an Error if any operation fails.
fn remove_seasonality(
    df: &mut DataFrame,
    start_date_timestamp: i64,
) -> Result<(Array1<f64>, Array1<f64>)> {
    let start_date = DateTime::from_timestamp(start_date_timestamp / 1000, 0)
        .ok_or_else(|| err!("Can't calculate the start date"))?;

    let t_series: Vec<f64> = df
        .column("date")?
        .datetime()?
        .into_iter()
        .map(|opt_date| {
            opt_date.map_or(0.0, |date| {
                (DateTime::from_timestamp(date / 1000, 0).unwrap() - start_date).num_seconds()
                    as f64
                    / 3600.0
            })
        })
        .collect();

    df.with_column(Series::new("t".into(), t_series))?;

    let t_array = df["t"].f64()?.to_ndarray()?.to_owned();
    let c = season_matrix(t_array);

    let detrended_log_base_fee_array = df["detrended_log_base_fee"].f64()?.to_ndarray()?.to_owned();
    let season_param = c.least_squares(&detrended_log_base_fee_array)?.solution;
    let season = c.dot(&season_param);
    let de_seasonalised_detrended_log_base_fee =
        df["detrended_log_base_fee"].f64()?.to_ndarray()?.to_owned() - season;

    Ok((de_seasonalised_detrended_log_base_fee, season_param))
}

/// Performs Monte Carlo parameter estimation and simulation for the Mean-Reverting Jump (MRJ) model.
///
/// This function estimates the parameters of the MRJ model using Monte Carlo methods,
/// and then uses these parameters to simulate future prices.
///
/// # Arguments
///
/// * `de_seasonalised_detrended_log_base_fee` - An array of de-seasonalized and de-trended log base fees.
/// * `n_periods` - The number of periods to simulate.
/// * `num_paths` - The number of simulation paths.
///
/// # Returns
///
/// A tuple containing:
/// * The simulated prices as a 2D array.
/// * The estimated model parameters.
///
/// # Errors
///
/// This function will return an error if:
/// * The parameter estimation fails.
/// * The Binomial distribution creation fails.
fn simulate_prices(
    de_seasonalised_detrended_log_base_fee: ArrayView1<f64>,
    n_periods: usize,
    num_paths: usize,
) -> Result<(Array2<f64>, Vec<f64>)> {
    let dt = 1.0 / (365.0 * 24.0);
    let pt = de_seasonalised_detrended_log_base_fee
        .slice(s![1..])
        .to_owned();
    let pt_1 = de_seasonalised_detrended_log_base_fee
        .slice(s![..-1])
        .to_owned();

    let function =
        NumericalDifferentiation::new(Func(|x: &[f64]| neg_log_likelihood(x, &pt, &pt_1)));

    let minimizer = GradientDescent::new().max_iterations(Some(2400));

    let var_pt = pt.var(0.0);
    let solution = minimizer.minimize(
        &function,
        vec![-3.928e-02, 2.873e-04, 4.617e-02, var_pt, var_pt, 0.2],
    );

    let params = &solution.position;
    let alpha = params[0] / dt;
    let kappa = (1.0 - params[1]) / dt;
    let mu_j = params[2];
    let sigma = (params[3] / dt).sqrt();
    let sigma_j = params[4].sqrt();
    let lambda_ = params[5] / dt;

    let mut rng = thread_rng();
    let j: Array2<f64> = {
        let binom = Binomial::new(lambda_ * dt, 1)?;
        Array2::from_shape_fn((n_periods, num_paths), |_| binom.sample(&mut rng) as f64)
    };

    let mut simulated_prices = Array2::zeros((n_periods, num_paths));
    simulated_prices
        .slice_mut(s![0, ..])
        .assign(&Array1::from_elem(
            num_paths,
            de_seasonalised_detrended_log_base_fee
                [de_seasonalised_detrended_log_base_fee.len() - 1],
        ));

    let normal = Normal::new(0.0, 1.0).unwrap();
    let n1 = Array2::from_shape_fn((n_periods, num_paths), |_| normal.sample(&mut rng));
    let n2 = Array2::from_shape_fn((n_periods, num_paths), |_| normal.sample(&mut rng));

    for i in 1..n_periods {
        let prev_prices = simulated_prices.slice(s![i - 1, ..]);
        let current_n1 = n1.slice(s![i, ..]);
        let current_n2 = n2.slice(s![i, ..]);
        let current_j = j.slice(s![i, ..]);

        let new_prices = &(alpha * dt
            + (1.0 - kappa * dt) * &prev_prices
            + sigma * dt.sqrt() * &current_n1
            + &current_j * (mu_j + sigma_j * &current_n2));

        simulated_prices
            .slice_mut(s![i, ..])
            .assign(&new_prices.clone());
    }

    Ok((simulated_prices, params.to_vec()))
}

/// Discovers the trend in the log base fee data using linear regression.
///
/// # Arguments
///
/// * `df` - A reference to a DataFrame containing the log base fee data.
///
/// # Returns
///
/// A Result containing a tuple with:
/// * The fitted linear regression model.
/// * A vector of trend values corresponding to the input data points.
///
/// # Errors
///
/// Returns an Error if:
/// * The 'log_base_fee' column cannot be accessed or converted to f64.
/// * The linear regression model fails to fit.
fn discover_trend(df: &DataFrame) -> Result<(FittedLinearRegression<f64>, Vec<f64>)> {
    let time_index: Vec<f64> = (0..df.height() as i64).map(|i| i as f64).collect();

    let ones = Array::<f64, Ix1>::ones(df.height());
    let x = stack![Axis(1), Array::from(time_index.clone()), ones];

    let y = Array1::from(
        df["log_base_fee"]
            .f64()?
            .into_no_null_iter()
            .collect::<Vec<f64>>(),
    );

    let dataset = Dataset::<f64, f64, Ix1>::new(x.clone(), y);
    let trend_model = LinearRegression::default()
        .with_intercept(false)
        .fit(&dataset)?;

    let trend_values = trend_model.predict(&x).as_targets().to_vec();

    Ok((trend_model, trend_values))
}

// Computes the natural logarithm of 'base_fee' values
fn compute_log_of_base_fees(df: &DataFrame) -> Result<Vec<f64>> {
    let log_base_fees: Vec<f64> = df
        .column("base_fee")?
        .f64()?
        .into_no_null_iter()
        .map(|x| x.ln())
        .collect();
    Ok(log_base_fees)
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

/// Creates a seasonal matrix for time series analysis.
///
/// This function generates a matrix of seasonal components based on the input time array.
/// It calculates various sine and cosine terms to capture daily and weekly seasonality patterns.
///
/// # Arguments
///
/// * `t` - An `Array1<f64>` representing the time points for which to generate the seasonal matrix.
///
/// # Returns
///
/// An `Array2<f64>` containing the seasonal components. Each column represents a different
/// seasonal term, and each row corresponds to a time point in the input array.
///
/// # Seasonal Components
///
/// The function calculates the following seasonal components:
/// - Daily components: sin(2πt/24), cos(2πt/24), sin(4πt/24), cos(4πt/24), sin(8πt/24), cos(8πt/24)
/// - Weekly components: sin(2πt/(24*7)), cos(2πt/(24*7)), sin(4πt/(24*7)), cos(4πt/(24*7)), sin(8πt/(24*7)), cos(8πt/(24*7))
///
fn season_matrix(t: Array1<f64>) -> Array2<f64> {
    let sin_2pi_24 = t.mapv(|time| (2.0 * PI * time / 24.0).sin());
    let cos_2pi_24 = t.mapv(|time| (2.0 * PI * time / 24.0).cos());
    let sin_4pi_24 = t.mapv(|time| (4.0 * PI * time / 24.0).sin());
    let cos_4pi_24 = t.mapv(|time| (4.0 * PI * time / 24.0).cos());
    let sin_8pi_24 = t.mapv(|time| (8.0 * PI * time / 24.0).sin());
    let cos_8pi_24 = t.mapv(|time| (8.0 * PI * time / 24.0).cos());
    let sin_2pi_24_7 = t.mapv(|time| (2.0 * PI * time / (24.0 * 7.0)).sin());
    let cos_2pi_24_7 = t.mapv(|time| (2.0 * PI * time / (24.0 * 7.0)).cos());
    let sin_4pi_24_7 = t.mapv(|time| (4.0 * PI * time / (24.0 * 7.0)).sin());
    let cos_4pi_24_7 = t.mapv(|time| (4.0 * PI * time / (24.0 * 7.0)).cos());
    let sin_8pi_24_7 = t.mapv(|time| (8.0 * PI * time / (24.0 * 7.0)).sin());
    let cos_8pi_24_7 = t.mapv(|time| (8.0 * PI * time / (24.0 * 7.0)).cos());

    stack![
        Axis(1),
        sin_2pi_24,
        cos_2pi_24,
        sin_4pi_24,
        cos_4pi_24,
        sin_8pi_24,
        cos_8pi_24,
        sin_2pi_24_7,
        cos_2pi_24_7,
        sin_4pi_24_7,
        cos_4pi_24_7,
        sin_8pi_24_7,
        cos_8pi_24_7
    ]
}

/// Calculates the standard deviation of a vector of floating-point numbers.
///
/// This function computes the sample standard deviation of the input vector.
/// It uses the n-1 formula for sample standard deviation, which is more
/// appropriate for estimating the standard deviation of a population
/// from a sample.
///
/// # Arguments
///
/// * `returns` - A vector of f64 values representing the data points.
///
/// # Returns
///
/// * `f64` - The calculated sample standard deviation.
///
/// # Notes
///
/// - This function uses the two-pass algorithm to compute the variance,
///   which can be more numerically stable for large datasets.
/// - If the input vector has fewer than two elements, the function will
///   return 0.0 to avoid division by zero.
fn standard_deviation(returns: Vec<f64>) -> f64 {
    let n = returns.len() as f64;
    if n < 2.0 {
        return 0.0; // Return 0 for vectors with less than 2 elements
    }
    let mean = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    variance.sqrt()
}

/// Calculates the probability density function (PDF) for the Mean-Reverting Jump (MRJ) model.
///
/// This function computes the PDF of the MRJ model given the model parameters and observed prices.
///
/// # Arguments
///
/// * `params` - A slice of f64 values representing the model parameters:
///   [a, phi, mu_j, sigma_sq, sigma_sq_j, lambda]
/// * `pt` - An Array1<f64> of observed prices at time t
/// * `pt_1` - An Array1<f64> of observed prices at time t-1
///
/// # Returns
///
/// * `Array1<f64>` - The calculated PDF values
///
/// # Notes
///
/// The MRJ model combines a mean-reverting process with a jump component. The PDF is a mixture
/// of two normal distributions, weighted by the jump probability (lambda).
fn mrjpdf(params: &[f64], pt: &Array1<f64>, pt_1: &Array1<f64>) -> Array1<f64> {
    let (a, phi, mu_j, sigma_sq, sigma_sq_j, lambda) = (
        params[0], params[1], params[2], params[3], params[4], params[5],
    );

    let term1 = lambda
        * (-((pt - a - phi * pt_1 - mu_j).mapv(|x| x.powi(2))) / (2.0 * (sigma_sq + sigma_sq_j)))
            .mapv(f64::exp)
        / ((2.0 * std::f64::consts::PI * (sigma_sq + sigma_sq_j)).sqrt());

    let term2 = (1.0 - lambda)
        * (-((pt - a - phi * pt_1).mapv(|x| x.powi(2))) / (2.0 * sigma_sq)).mapv(f64::exp)
        / ((2.0 * std::f64::consts::PI * sigma_sq).sqrt());

    term1 + term2
}

/// Calculates the negative log-likelihood for the mean-reverting jump diffusion model.
///
/// This function computes the negative log-likelihood of the observed data given the model parameters.
/// It's used in parameter estimation for the mean-reverting jump diffusion model.
///
/// # Arguments
///
/// * `params` - A slice of f64 values representing the model parameters:
///   [a, phi, mu_j, sigma_sq, sigma_sq_j, lambda]
/// * `pt` - An Array1<f64> of observed prices at time t
/// * `pt_1` - An Array1<f64> of observed prices at time t-1
///
/// # Returns
///
/// * `f64` - The negative log-likelihood value
///
/// # Notes
///
/// The function adds a small constant (1e-10) to each PDF value before taking the logarithm
/// to avoid potential issues with zero values.
fn neg_log_likelihood(params: &[f64], pt: &Array1<f64>, pt_1: &Array1<f64>) -> f64 {
    let pdf_vals = mrjpdf(params, pt, pt_1);
    -pdf_vals.mapv(|x| (x + 1e-10).ln()).sum()
}

/// Adds a Time-Weighted Average Price (TWAP) column to the DataFrame.
///
/// This function calculates the 7-day TWAP for the 'base_fee' column and adds it as a new column
/// named 'TWAP_7d' to the input DataFrame.
///
/// # Arguments
///
/// * `df` - The input DataFrame containing the 'base_fee' column.
///
/// # Returns
///
/// A `Result` containing the DataFrame with the added 'TWAP_7d' column, or an `Error` if the
/// operation fails.
///
/// # Errors
///
/// This function will return an error if:
/// * The rolling mean calculation fails.
/// * The final collection of the lazy DataFrame fails.
///
fn add_twap_7d(df: DataFrame) -> Result<DataFrame> {
    let required_window_size = 24 * 7;

    tracing::debug!("DataFrame shape before TWAP: {:?}", df.shape());
    tracing::debug!("df height: {}", df.height());

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
            .alias("TWAP_7d"),
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
// Instead of grouping, just sort the data
fn group_by_1h_intervals(df: DataFrame) -> Result<DataFrame> {
    tracing::debug!("DataFrame shape before sorting: {:?}", df.shape());

    let df = df
        .lazy()
        .sort(
            vec!["date"],
            SortMultipleOptions {
                descending: vec![false],
                nulls_last: vec![true],
                multithreaded: true,
                maintain_order: false,
            },
        )
        .collect()?;

    tracing::debug!("DataFrame shape after sorting: {:?}", df.shape());
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
    tracing::debug!("Timestamp column: {:?}", df.column("timestamp"));

    // Ensure the column is of type String
    let timestamp_col = df.column("timestamp")?;
    if timestamp_col.dtype() != &DataType::String {
        return Err(eyre::eyre!("Timestamp column is not of type String"));
    }

    // Parse hex-encoded timestamps into i64 values
    let parsed_timestamps: Vec<i64> = timestamp_col
        .str()? // Use str() instead of utf8()
        .into_iter()
        .map(|opt| opt.and_then(|s| i64::from_str_radix(s.trim_start_matches("0x"), 16).ok()))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| eyre::eyre!("Failed to parse timestamps"))?;

    tracing::debug!("Parsed timestamps: {:?}", parsed_timestamps);

    // Convert to milliseconds and create a Series
    let dates = Series::new(
        "date".into(),
        parsed_timestamps
            .into_iter()
            .map(|ts| ts * 1000) // Convert seconds to milliseconds
            .collect::<Vec<_>>(),
    )
    .cast(&DataType::Datetime(TimeUnit::Milliseconds, None))?;

    tracing::debug!("Dates series: {:?}", dates);

    // Create a new DataFrame with the date column replacing timestamp
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

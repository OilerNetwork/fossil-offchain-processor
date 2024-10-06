use axum::{extract::State, http::StatusCode, Json};
use db_access::{queries::get_block_headers_by_time_range, DbConnection};
use serde::{Deserialize, Serialize};
use starknet_crypto::{poseidon_hash_single, Felt};
use tokio::{join, time::Instant};
use tracing::{error, info};

use crate::pricing_data::{
    reserve_price::calculate_reserve_price, twap::calculate_twap, volatility::calculate_volatility,
};

// timestamp ranges for each sub-job calculation
#[derive(Debug, Deserialize, Serialize)]
pub struct PitchLakeJobRequestParams {
    twap: (i64, i64),
    volatility: (i64, i64),
    reserve_price: (i64, i64),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PitchLakeJobRequest {
    identifiers: Vec<String>,
    params: PitchLakeJobRequestParams,
    callback_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PitchLakeJobCallback {
    Fail(PitchLakeJobFailedCallback),
    Success(PitchLakeJobSuccessCallback),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PitchLakeJobSuccessCallback {
    pub job_id: String,
    pub twap: f64,
    pub volatility: f64,
    pub reserve_price: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PitchLakeJobFailedCallback {
    pub job_id: String,
    pub error: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JobResponse {
    job_id: String,
}

pub async fn get_pricing_data(
    State(db): State<DbConnection>,
    Json(payload): Json<PitchLakeJobRequest>,
) -> (StatusCode, Json<JobResponse>) {
    let job_id = poseidon_hash_single(Felt::from_bytes_be_slice(
        payload.identifiers.join("").as_bytes(),
    ));

    // Spawn async task for processing the job
    tokio::spawn(perform_calculations_and_callback(db, payload, job_id));

    (
        StatusCode::OK,
        Json(JobResponse {
            job_id: job_id.to_string(),
        }),
    )
}

async fn perform_calculations_and_callback(
    db: DbConnection,
    payload: PitchLakeJobRequest,
    job_id: Felt,
) {
    // Fetch block headers concurrently
    let block_headers_for_calculations = join!(
        get_block_headers_by_time_range(&db.pool, payload.params.twap.0, payload.params.twap.1),
        get_block_headers_by_time_range(
            &db.pool,
            payload.params.volatility.0,
            payload.params.volatility.1
        ),
        get_block_headers_by_time_range(
            &db.pool,
            payload.params.reserve_price.0,
            payload.params.reserve_price.1
        )
    );

    let (twap_blockheaders, volatility_blockheaders, reserve_price_blockheaders) =
        match block_headers_for_calculations {
            (
                Ok(twap_blockheaders),
                Ok(volatility_blockheaders),
                Ok(reserve_price_blockheaders),
            ) => (
                twap_blockheaders,
                volatility_blockheaders,
                reserve_price_blockheaders,
            ),
            _ => {
                error!(
                    "Failed to query db data: {:?}",
                    block_headers_for_calculations
                );
                send_failure_callback(
                    payload.callback_url.clone(),
                    job_id.to_string(),
                    "Failed to query DB data".to_string(),
                )
                .await;
                return;
            }
        };

    // Run computations concurrently
    let twap_future = calculate_twap(twap_blockheaders);
    let volatility_future = calculate_volatility(volatility_blockheaders);
    let reserve_price_future = calculate_reserve_price(reserve_price_blockheaders);

    let now = Instant::now();
    info!("Started processing job_id={}", job_id);

    let futures_result = join!(twap_future, volatility_future, reserve_price_future);

    let elapsed = now.elapsed();
    info!("Job_id={} processing completed in {:.2?}", job_id, elapsed);

    let client = reqwest::Client::new();
    let callback_url = payload.callback_url.clone();

    match futures_result {
        (Ok(twap), Ok(volatility_result), Ok(reserve_price_result)) => {
            info!(
                "TWAP result for job_id={}: {:?}, Volatility result: {:?}, Reserve price result: {:?}",
                job_id, twap, volatility_result, reserve_price_result
            );

            let res = client
                .post(callback_url.clone())
                .json(&PitchLakeJobSuccessCallback {
                    job_id: job_id.to_string(),
                    twap,
                    volatility: volatility_result,
                    reserve_price: reserve_price_result,
                })
                .send()
                .await;

            if let Err(err) = handle_callback_response(res, &job_id.to_string()).await {
                error!("Callback failed for job_id={}: {}", job_id, err);
            }
        }
        _ => {
            error!(
                "Failed calculation for job_id={}: {:?}",
                job_id, futures_result
            );

            send_failure_callback(
                callback_url.clone(),
                job_id.to_string(),
                "Failed to calculate data".to_string(),
            )
            .await;
        }
    }
}

async fn send_failure_callback(callback_url: String, job_id: String, error: String) {
    let client = reqwest::Client::new();
    let res = client
        .post(callback_url.clone())
        .json(&PitchLakeJobFailedCallback { job_id, error })
        .send()
        .await;

    if let Err(err) = handle_callback_response(res, &job_id).await {
        error!(
            "Failed to send failure callback for job_id={}: {}",
            job_id, err
        );
    }
}

async fn handle_callback_response(
    res: Result<reqwest::Response, reqwest::Error>,
    job_id: &str,
) -> Result<(), String> {
    match res {
        Ok(callback_res) => {
            if !callback_res.status().is_success() {
                let text = callback_res
                    .text()
                    .await
                    .unwrap_or_else(|_| "Failed to retrieve error text".to_string());
                Err(format!(
                    "Callback response unsuccessful for job_id={}: {}",
                    job_id, text
                ))
            } else {
                Ok(())
            }
        }
        Err(err) => Err(format!(
            "Callback call failed for job_id={}: {}",
            job_id, err
        )),
    }
}

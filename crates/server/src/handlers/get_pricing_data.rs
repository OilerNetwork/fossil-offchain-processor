use axum::{extract::State, http::StatusCode, Json};
use db_access::{queries::get_block_headers_by_time_range, DbConnection};
use serde::{Deserialize, Serialize};
use starknet_crypto::{poseidon_hash_single, Felt};
use std::collections::HashMap;
use tokio::{join, time::Instant};

use crate::pricing_data::{twap::calculate_twap, volatility::calculate_volatility};

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

// TODO: Placeholder for now, need to be more generic
// Note that all the number fields are f64; this is because
// json supports only i32 and for some reason f64 when deserializing.
// If we want values other than these, making them string might be a
// good idea.
#[derive(Debug, Deserialize, Serialize)]
pub struct PitchLakeJobSuccessCallback {
    pub job_id: String,
    pub twap: HashMap<String, f64>,
    pub volatility: f64,
    pub reserve_price: f64,
}

/// TODO: might want to introduce a 'status' field or
/// an 'error code' field to make error handling on client
/// side better.
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
    // TODO(cwk): save the jobid somewhere

    // TODO: Is there anyway to extract this async section?
    tokio::spawn(async move {
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

        let (twap_blockheaders, volatility_blockheaders, _) = match block_headers_for_calculations {
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
                // If there's a failure in querying, do not exit peacefully.
                // it means there's something wrong with our db queries.
                // TOOD: add more detailed error handling.
                panic!(
                    "Fail to query db data: {:?}",
                    block_headers_for_calculations
                );
            }
        };

        let twap_future = calculate_twap(twap_blockheaders);
        let volatility_future = calculate_volatility(volatility_blockheaders);

        let now = Instant::now();
        println!("Started processing...");

        let futures_result = join!(twap_future, volatility_future);

        let elapsed = now.elapsed();
        println!("Elapsed: {:.2?}", elapsed);

        let client = reqwest::Client::new();

        let callback_url = payload.callback_url.clone();

        match futures_result {
            (Ok(twap), Ok(volatility_result)) => {
                // callback the result of the calculation to a given callback url.
                let res = client
                    .post(callback_url.clone())
                    .json(&PitchLakeJobSuccessCallback {
                        job_id: job_id.to_string(),
                        twap,
                        volatility: volatility_result,
                        reserve_price: 0f64,
                    })
                    .send()
                    .await;

                match res {
                    // If our callback fail we can't do much to inform our client.
                    // so just log out the issue for debugging.
                    Ok(callback_res) => {
                        if !callback_res.status().is_success() {
                            eprintln!(
                                "Callback response unsuccessful: {:?}",
                                callback_res.text().await
                            );
                        }
                    }
                    Err(err) => {
                        eprintln!("Callback call failed: {}", err);
                    }
                }
            }
            // This means that either some of them failed, or all of them failed.
            // treating a single fail as all failed for now.
            future_tuple_with_err => {
                // We try to also inform of calculation error, so that client side knows
                // when there's an issue with calculation on our part

                eprintln!("Failed calculation: {:?}", future_tuple_with_err);

                let res = client
                    .post(callback_url.clone())
                    .json(&PitchLakeJobFailedCallback {
                        job_id: job_id.to_string(),
                        error: "Failed to calculate data".to_string(),
                    })
                    .send()
                    .await;

                match res {
                    // If our callback fail we can't do much to inform our client.
                    // so just log out the issue for debugging.
                    Ok(callback_res) => {
                        if !callback_res.status().is_success() {
                            eprintln!("Callback response unsuccessful: {:?}", callback_res);
                        }
                    }
                    Err(err) => {
                        eprintln!("Callback call failed: {}", err);
                    }
                }
            }
        }
    });

    (
        StatusCode::OK,
        Json(JobResponse {
            job_id: job_id.to_string(),
        }),
    )
}

use axum::{extract::State, http::StatusCode, Json};
use db_access::{
    models::JobStatus,
    queries::{
        create_job_request, get_block_headers_by_time_range, get_job_request, update_job_status,
    },
    DbConnection,
};
use starknet_crypto::{poseidon_hash_single, Felt};
use tokio::{join, time::Instant};

use crate::pricing_data::{
    reserve_price::calculate_reserve_price, twap::calculate_twap, volatility::calculate_volatility,
};
use crate::types::{JobResponse, PitchLakeJobRequest};

pub async fn get_pricing_data(
    State(db): State<DbConnection>,
    Json(payload): Json<PitchLakeJobRequest>,
) -> (StatusCode, Json<JobResponse>) {
    let job_id = poseidon_hash_single(Felt::from_bytes_be_slice(
        payload.identifiers.join("").as_bytes(),
    ))
    .to_string();

    match get_job_request(&db.pool, &job_id).await {
        Ok(Some(job_request)) => match job_request.status {
            JobStatus::Pending => (
                StatusCode::CONFLICT,
                Json(JobResponse {
                    job_id: job_id.clone(),
                    message: "Job is already pending. Use the status endpoint to monitor progress."
                        .to_string(),
                    status_url: format!("/job_status/{}", job_id),
                }),
            ),
            JobStatus::Completed => (
                StatusCode::CONFLICT,
                Json(JobResponse {
                    job_id: job_id.clone(),
                    message: "Job has already been completed. No further processing required."
                        .to_string(),
                    status_url: format!("/job_status/{}", job_id),
                }),
            ),
            JobStatus::Failed => {
                // Reprocess the failed job
                if let Err(e) = update_job_status(&db.pool, &job_id, JobStatus::Pending).await {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(JobResponse {
                        job_id: job_id.clone(),
                        message: format!("Previous job request failed. An error occurred while updating job status: {}", e).to_string(),
                        status_url: format!("/job_status/{}", job_id),
                    }));
                }
                tokio::spawn(process_job(db.clone(), job_id.clone(), payload));

                (
                    StatusCode::OK,
                    Json(JobResponse {
                        job_id: job_id.clone(),
                        message: "Previous job request failed. Reprocessing initiated.".to_string(),
                        status_url: format!("/job_status/{}", job_id),
                    }),
                )
            }
        },
        Ok(None) => {
            // New job
            if let Err(e) = create_job_request(&db.pool, &job_id, JobStatus::Pending).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(JobResponse {
                        job_id: job_id.clone(),
                        message: format!("An error occurred while creating the job: {}", e)
                            .to_string(),
                        status_url: format!("/job_status/{}", job_id),
                    }),
                );
            }
            tokio::spawn(process_job(db.clone(), job_id.clone(), payload));

            (
                StatusCode::CREATED,
                Json(JobResponse {
                    job_id: job_id.clone(),
                    message: "New job request registered and processing initiated.".to_string(),
                    status_url: format!("/job_status/{}", job_id),
                }),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(JobResponse {
                job_id: job_id.clone(),
                message: format!("An error occurred while processing the request: {}", e)
                    .to_string(),
                status_url: format!("/job_status/{}", job_id),
            }),
        ),
    }
}

async fn process_job(db: DbConnection, job_id: String, payload: PitchLakeJobRequest) {
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
                tracing::error!(
                    "Failed to query db data: {:?}",
                    block_headers_for_calculations
                );
                let _ = update_job_status(&db.pool, &job_id, JobStatus::Failed).await;
                return;
            }
        };

    let twap_future = calculate_twap(twap_blockheaders);
    let volatility_future = calculate_volatility(volatility_blockheaders);
    let reserve_price_future = calculate_reserve_price(reserve_price_blockheaders);

    let now = Instant::now();
    tracing::info!("Started processing...");

    let futures_result = join!(twap_future, volatility_future, reserve_price_future);

    let elapsed = now.elapsed();
    tracing::info!("Elapsed: {:.2?}", elapsed);

    match futures_result {
        (Ok(twap), Ok(volatility_result), Ok(reserve_price_result)) => {
            tracing::debug!("TWAP result: {:?}", twap);
            tracing::debug!("Volatility result: {:?}", volatility_result);
            tracing::debug!("Reserve price result: {:?}", reserve_price_result);
            let _ = update_job_status(&db.pool, &job_id, JobStatus::Completed).await;
            // TODO: Send success callback
        }
        future_tuple_with_err => {
            tracing::error!("Failed calculation: {:?}", future_tuple_with_err);
            let _ = update_job_status(&db.pool, &job_id, JobStatus::Failed).await;
            // TODO: Send failure callback
        }
    }
}

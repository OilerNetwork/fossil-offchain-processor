use std::sync::Arc;

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

use crate::types::{JobResponse, PitchLakeJobRequest};
use crate::{
    pricing_data::{
        reserve_price::calculate_reserve_price, twap::calculate_twap,
        volatility::calculate_volatility,
    },
    types::PitchLakeJobRequestParams,
};

use crate::AppState;

pub async fn get_pricing_data(
    State(state): State<AppState>,
    Json(payload): Json<PitchLakeJobRequest>,
) -> (StatusCode, Json<JobResponse>) {
    if payload.identifiers.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(JobResponse {
                job_id: String::new(),
                message: "Identifiers cannot be empty.".to_string(),
                status_url: String::new(),
            }),
        );
    }

    if let Err((status, response)) = validate_time_ranges(&payload.params) {
        return (status, Json(response));
    }

    let job_id = poseidon_hash_single(Felt::from_bytes_be_slice(
        payload.identifiers.join("").as_bytes(),
    ))
    .to_string();

    match get_job_request(&state.db.pool, &job_id).await {
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
                if let Err(e) = update_job_status(&state.db.pool, &job_id, JobStatus::Pending).await
                {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(JobResponse {
                        job_id: job_id.clone(),
                        message: format!("Previous job request failed. An error occurred while updating job status: {}", e).to_string(),
                        status_url: format!("/job_status/{}", job_id),
                    }));
                }
                tokio::spawn(process_job(state.db.clone(), job_id.clone(), payload));

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
            match create_job_request(&state.db.pool, &job_id, JobStatus::Pending).await {
                Ok(_) => {
                    tokio::spawn(process_job(state.db.clone(), job_id.clone(), payload));

                    (
                        StatusCode::CREATED,
                        Json(JobResponse {
                            job_id: job_id.clone(),
                            message: "New job request registered and processing initiated."
                                .to_string(),
                            status_url: format!("/job_status/{}", job_id),
                        }),
                    )
                }
                Err(e) => {
                    tracing::error!("Failed to create job request: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(JobResponse {
                            job_id: job_id.clone(),
                            message: format!("An error occurred while creating the job: {}", e),
                            status_url: format!("/job_status/{}", job_id),
                        }),
                    )
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to get job request: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(JobResponse {
                    job_id: job_id.clone(),
                    message: format!("An error occurred while processing the request: {}", e),
                    status_url: format!("/job_status/{}", job_id),
                }),
            )
        }
    }
}

async fn process_job(db: Arc<DbConnection>, job_id: String, payload: PitchLakeJobRequest) {
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

fn validate_time_ranges(
    params: &PitchLakeJobRequestParams,
) -> Result<(), (StatusCode, JobResponse)> {
    let validations = [
        ("TWAP", params.twap),
        ("volatility", params.volatility),
        ("reserve price", params.reserve_price),
    ];

    for &(name, (start, end)) in validations.iter() {
        if start >= end {
            return Err((
                StatusCode::BAD_REQUEST,
                JobResponse {
                    job_id: String::new(),
                    message: format!("Invalid time range for {} calculation.", name),
                    status_url: String::new(),
                },
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::fixtures::TestContext;
    use crate::types::{PitchLakeJobRequest, PitchLakeJobRequestParams};
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_get_pricing_data_new_job() {
        let ctx = TestContext::new().await;

        let payload = PitchLakeJobRequest {
            identifiers: vec!["test-id".to_string()],
            params: PitchLakeJobRequestParams {
                twap: (0, 100),
                volatility: (0, 100),
                reserve_price: (0, 100),
            },
        };

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::CREATED);
        assert!(!response.job_id.is_empty());
        assert_eq!(
            response.message,
            "New job request registered and processing initiated."
        );
        assert!(response.status_url.starts_with("/job_status/"));
    }

    #[tokio::test]
    async fn test_get_pricing_data_pending_job() {
        let ctx = TestContext::new().await;

        let job_id =
            poseidon_hash_single(Felt::from_bytes_be_slice("test-id".as_bytes())).to_string();
        ctx.create_job(&job_id, JobStatus::Pending).await;

        let payload = PitchLakeJobRequest {
            identifiers: vec!["test-id".to_string()],
            params: PitchLakeJobRequestParams {
                twap: (0, 100),
                volatility: (0, 100),
                reserve_price: (0, 100),
            },
        };

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(response.job_id, job_id);
        assert_eq!(
            response.message,
            "Job is already pending. Use the status endpoint to monitor progress."
        );
        assert_eq!(response.status_url, format!("/job_status/{}", job_id));
    }

    #[tokio::test]
    async fn test_get_pricing_data_completed_job() {
        let ctx = TestContext::new().await;

        let job_id =
            poseidon_hash_single(Felt::from_bytes_be_slice("test-id".as_bytes())).to_string();
        ctx.create_job(&job_id, JobStatus::Completed).await;

        let payload = PitchLakeJobRequest {
            identifiers: vec!["test-id".to_string()],
            params: PitchLakeJobRequestParams {
                twap: (0, 100),
                volatility: (0, 100),
                reserve_price: (0, 100),
            },
        };

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(response.job_id, job_id);
        assert_eq!(
            response.message,
            "Job has already been completed. No further processing required."
        );
        assert_eq!(response.status_url, format!("/job_status/{}", job_id));
    }

    #[tokio::test]
    async fn test_get_pricing_data_failed_job() {
        let ctx = TestContext::new().await;

        let job_id =
            poseidon_hash_single(Felt::from_bytes_be_slice("test-id".as_bytes())).to_string();
        ctx.create_job(&job_id, JobStatus::Failed).await;

        let payload = PitchLakeJobRequest {
            identifiers: vec!["test-id".to_string()],
            params: PitchLakeJobRequestParams {
                twap: (0, 100),
                volatility: (0, 100),
                reserve_price: (0, 100),
            },
        };

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.job_id, job_id);
        assert_eq!(
            response.message,
            "Previous job request failed. Reprocessing initiated."
        );
        assert_eq!(response.status_url, format!("/job_status/{}", job_id));

        // Verify that the job status was updated to Pending
        let (_, Json(status_response)) = ctx.get_job_status(&job_id).await;
        assert_eq!(status_response["status"], "Pending");
    }

    #[tokio::test]
    async fn test_get_pricing_data_multiple_identifiers() {
        let ctx = TestContext::new().await;

        let payload = PitchLakeJobRequest {
            identifiers: vec!["id1".to_string(), "id2".to_string(), "id3".to_string()],
            params: PitchLakeJobRequestParams {
                twap: (0, 100),
                volatility: (0, 100),
                reserve_price: (0, 100),
            },
        };

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::CREATED);
        assert!(!response.job_id.is_empty());
        assert_eq!(
            response.message,
            "New job request registered and processing initiated."
        );
        assert!(response.status_url.starts_with("/job_status/"));

        // Verify that the job_id is a hash of all identifiers
        let expected_job_id =
            poseidon_hash_single(Felt::from_bytes_be_slice("id1id2id3".as_bytes())).to_string();
        assert_eq!(response.job_id, expected_job_id);
    }

    #[tokio::test]
    async fn test_get_pricing_data_empty_identifiers() {
        let ctx = TestContext::new().await;

        let payload = PitchLakeJobRequest {
            identifiers: vec![],
            params: PitchLakeJobRequestParams {
                twap: (0, 100), // Invalid range
                volatility: (0, 100),
                reserve_price: (0, 100),
            },
        };

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(response.message, "Identifiers cannot be empty.");
        assert!(response.job_id.is_empty());
        assert!(response.status_url.is_empty());
    }

    #[tokio::test]
    async fn test_get_pricing_data_invalid_params() {
        let ctx = TestContext::new().await;

        let payload = PitchLakeJobRequest {
            identifiers: vec!["test-id".to_string()],
            params: PitchLakeJobRequestParams {
                twap: (100, 0), // Invalid range
                volatility: (0, 100),
                reserve_price: (0, 100),
            },
        };

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(response.message, "Invalid time range for TWAP calculation.");
        assert!(response.job_id.is_empty());
        assert!(response.status_url.is_empty());
    }
}

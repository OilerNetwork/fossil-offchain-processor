use std::sync::Arc;

use axum::{
    extract::{Json, State},
    http::StatusCode,
};

use db_access::{
    models::JobStatus,
    queries::{
        create_job_request, get_block_headers_by_time_range, get_job_request, update_job_status,
    },
    DbConnection,
};
use starknet_crypto::{poseidon_hash_single, Felt};
use tokio::join;
use serde_json::json;

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
    tracing::debug!("Received payload: {:?}", payload);

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
        tracing::error!("Invalid time ranges: {:?}", payload.params);
        return (status, Json(response));
    }

    let job_id = poseidon_hash_single(Felt::from_bytes_be_slice(
        payload.identifiers.join("").as_bytes(),
    ))
    .to_string();

    let base_url = "http://localhost:3000".to_string();

    match get_job_request(&state.db.pool, &job_id).await {
        Ok(Some(job_request)) => match job_request.status {
            JobStatus::Pending => {
                let response = JobResponse {
                    job_id: job_id.clone(),
                    message: "Job is already pending. Use the status endpoint to monitor progress."
                        .to_string(),
                    status_url: format!("{}/job_status/{}", base_url, job_id),
                };
                (StatusCode::CONFLICT, Json(response))
            }
            JobStatus::Completed => {
                let response = JobResponse {
                    job_id: job_id.clone(),
                    message: "Job completed. Fetch the results from the status endpoint."
                        .to_string(),
                    status_url: format!("{}/job_status/{}", base_url, job_id),
                };
                (StatusCode::OK, Json(response))
            }
            JobStatus::Failed => {
                if let Err(e) =
                    update_job_status(&state.db.pool, &job_id, JobStatus::Pending, None).await
                {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(JobResponse {
                            job_id: job_id.clone(),
                            message: format!("Failed to update job status: {}", e),
                            status_url: format!("{}/job_status/{}", base_url, job_id),
                        }),
                    );
                }
                tokio::spawn(process_job(state.db.clone(), job_id.clone(), payload));
                (
                    StatusCode::OK,
                    Json(JobResponse {
                        job_id: job_id.clone(),
                        message: "Reprocessing initiated.".to_string(),
                        status_url: format!("{}/job_status/{}", base_url, job_id.clone()),
                    }),
                )
            }
        },
        Ok(None) => match create_job_request(&state.db.pool, &job_id, JobStatus::Pending).await {
            Ok(_) => {
                tokio::spawn(process_job(state.db.clone(), job_id.clone(), payload));
                (
                    StatusCode::CREATED,
                    Json(JobResponse {
                        job_id: job_id.clone(),
                        message: "Processing initiated.".to_string(),
                        status_url: format!("{}/job_status/{}", base_url, job_id),
                    }),
                )
            }
            Err(e) => {
                tracing::error!("Failed to create job: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(JobResponse {
                        job_id: job_id.clone(),
                        message: format!("Error creating job: {}", e),
                        status_url: format!("{}/job_status/{}", base_url, job_id.clone()),
                    }),
                )
            }
        },
        Err(e) => {
            tracing::error!("Error retrieving job: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(JobResponse {
                    job_id: job_id.clone(),
                    message: format!("Error retrieving job: {}", e),
                    status_url: format!("{}/job_status/{}", base_url, job_id.clone()),
                }),
            )
        }
    }
}

async fn process_job(db: Arc<DbConnection>, job_id: String, payload: PitchLakeJobRequest) {
    let block_headers = join!(
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

    let (twap_headers, volatility_headers, reserve_price_headers) = match block_headers {
        (Ok(twap), Ok(volatility), Ok(reserve_price)) => (twap, volatility, reserve_price),
        _ => {
            tracing::error!("Error fetching block headers.");
            let _ = update_job_status(&db.pool, &job_id, JobStatus::Failed, None).await;
            return;
        }
    };

    let results = join!(
        calculate_twap(twap_headers),
        calculate_volatility(volatility_headers),
        calculate_reserve_price(reserve_price_headers)
    );

    match results {
        (Ok(twap), Ok(volatility), Ok(reserve_price)) => {
            let result = json!({
                "twap": twap,
                "volatility": volatility,
                "reserve_price": reserve_price,
            });

            if let Err(e) =
                update_job_status(&db.pool, &job_id, JobStatus::Completed, Some(result)).await
            {
                tracing::error!("Error updating job status: {}", e);
            }
            tracing::info!("Job {} completed successfully.", job_id);
        }
        _ => {
            tracing::error!("Job {} failed.", job_id);
            let _ = update_job_status(&db.pool, &job_id, JobStatus::Failed, None).await;
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
            "Processing initiated."
        );
        assert_eq!(
            response.status_url,
            format!("http://localhost:3000/job_status/{}", response.job_id)
        );
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
        assert_eq!(
            response.status_url,
            format!("http://localhost:3000/job_status/{}", job_id)
        );
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

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.job_id, job_id);
        assert_eq!(
            response.message,
            "Job completed. Fetch the results from the status endpoint."
        );
        // TODO: Temporary fix for the test
        assert_eq!(
            response.status_url,
            format!("http://localhost:3000/job_status/{}", job_id)
        );
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
            "Reprocessing initiated."
        );
        assert_eq!(
            response.status_url,
            format!("http://localhost:3000/job_status/{}", job_id)
        );

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
            "Processing initiated."
        );
        assert_eq!(
            response.status_url,
            format!("http://localhost:3000/job_status/{}", response.job_id)
        );

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

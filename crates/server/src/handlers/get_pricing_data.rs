use std::sync::Arc;

use axum::{
    extract::{Json, State},
    http::StatusCode,
};

use crate::types::{JobResponse, PitchLakeJobRequest};
use crate::AppState;
use crate::{
    pricing_data::{
        reserve_price::calculate_reserve_price, twap::calculate_twap,
        volatility::calculate_volatility,
    },
    types::PitchLakeJobRequestParams,
};
use db_access::{
    models::JobStatus,
    queries::{
        create_job_request, get_block_headers_by_time_range, get_job_request, update_job_status,
    },
    DbConnection,
};
use serde_json::json;
use starknet_crypto::{poseidon_hash_single, Felt};
use tokio::join;

pub async fn get_pricing_data(
    State(state): State<AppState>,
    Json(payload): Json<PitchLakeJobRequest>,
) -> (StatusCode, Json<JobResponse>) {
    tracing::info!("Received pricing data request.");

    if let Err((status, response)) = validate_request(&payload) {
        tracing::warn!("Invalid request: {:?}", response);
        return (status, Json(response));
    }

    let job_id = generate_job_id(&payload.identifiers);
    tracing::info!("Generated job_id: {}", job_id);

    match get_job_request(&state.db.pool, &job_id).await {
        Ok(Some(job_request)) => {
            tracing::info!(
                "Handling existing job with status: {:?}",
                job_request.status
            );
            handle_existing_job(&state, job_request.status, job_id, payload).await
        }
        Ok(None) => {
            tracing::info!("Creating new job request.");
            handle_new_job_request(&state, job_id, payload).await
        }
        Err(e) => internal_server_error(e, job_id),
    }
}

// Helper to validate the incoming request
fn validate_request(payload: &PitchLakeJobRequest) -> Result<(), (StatusCode, JobResponse)> {
    if payload.identifiers.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            JobResponse {
                job_id: String::new(),
                message: "Identifiers cannot be empty.".to_string(),
                status_url: String::new(),
            },
        ));
    }
    validate_time_ranges(&payload.params)
}

// Helper to generate a job ID
fn generate_job_id(identifiers: &[String]) -> String {
    poseidon_hash_single(Felt::from_bytes_be_slice(identifiers.join("").as_bytes())).to_string()
}

// Handle existing jobs based on status
async fn handle_existing_job(
    state: &AppState,
    status: JobStatus,
    job_id: String,
    payload: PitchLakeJobRequest,
) -> (StatusCode, Json<JobResponse>) {
    match status {
        JobStatus::Pending => {
            tracing::info!("Job {} is already pending.", job_id);
            job_response(StatusCode::CONFLICT, job_id, "Job is already pending.")
        }
        JobStatus::Completed => {
            tracing::info!("Job {} completed. Returning results.", job_id);
            job_response(
                StatusCode::OK,
                job_id,
                "Job completed. Fetch results from the status endpoint.",
            )
        }
        JobStatus::Failed => {
            tracing::info!("Reprocessing failed job {}", job_id);
            reprocess_failed_job(state, job_id, payload).await
        }
    }
}

// Helper to handle a new job request
async fn handle_new_job_request(
    state: &AppState,
    job_id: String,
    payload: PitchLakeJobRequest,
) -> (StatusCode, Json<JobResponse>) {
    match create_job_request(&state.db.pool, &job_id, JobStatus::Pending).await {
        Ok(_) => {
            tracing::info!("Job {} created. Starting processing.", job_id);
            tokio::spawn(process_job(state.db.clone(), job_id.clone(), payload));
            job_response(StatusCode::CREATED, job_id, "Processing initiated.")
        }
        Err(e) => internal_server_error(e, job_id),
    }
}

// Helper to handle failed job reprocessing
async fn reprocess_failed_job(
    state: &AppState,
    job_id: String,
    payload: PitchLakeJobRequest,
) -> (StatusCode, Json<JobResponse>) {
    if let Err(e) = update_job_status(&state.db.pool, &job_id, JobStatus::Pending, None).await {
        return internal_server_error(e, job_id);
    }
    tokio::spawn(process_job(state.db.clone(), job_id.clone(), payload));
    job_response(StatusCode::OK, job_id, "Reprocessing initiated.")
}

// Generate a JSON job response
fn job_response(
    status: StatusCode,
    job_id: String,
    message: &str,
) -> (StatusCode, Json<JobResponse>) {
    tracing::info!("Responding to job {} with status {}", job_id, status);
    (
        status,
        Json(JobResponse {
            job_id: job_id.clone(),
            message: message.to_string(),
            status_url: format!("/job_status/{}", job_id),
        }),
    )
}

// Handle internal server errors
fn internal_server_error(error: sqlx::Error, job_id: String) -> (StatusCode, Json<JobResponse>) {
    tracing::error!("Internal server error: {:?}", error);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(JobResponse {
            job_id: job_id.clone(),
            message: format!("An error occurred: {}", error),
            status_url: format!("/job_status/{}", job_id),
        }),
    )
}
// Simplified job processing logic
async fn process_job(db: Arc<DbConnection>, job_id: String, payload: PitchLakeJobRequest) {
    tracing::info!("Processing job {}", job_id);

    match fetch_headers(&db, &payload).await {
        Some((twap, volatility, reserve_price)) => {
            let result = json!({
                "twap": twap,
                "volatility": volatility,
                "reserve_price": reserve_price,
            });

            if update_job_status(&db.pool, &job_id, JobStatus::Completed, Some(result))
                .await
                .is_err()
            {
                tracing::error!("Failed to update job status for {}", job_id);
            }
        }
        None => {
            tracing::error!("Failed to fetch headers for job {}", job_id);
            let _ = update_job_status(&db.pool, &job_id, JobStatus::Failed, None).await;
        }
    }
    tracing::info!("Job {} processing finished.", job_id);
}

// Helper to fetch block headers in parallel
async fn fetch_headers(
    db: &Arc<DbConnection>,
    payload: &PitchLakeJobRequest,
) -> Option<(f64, f64, f64)> {
    tracing::debug!("Fetching block headers for calculations.");

    let (twap_headers, volatility_headers, reserve_price_headers) = join!(
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

    match (twap_headers, volatility_headers, reserve_price_headers) {
        (Ok(twap), Ok(volatility), Ok(reserve)) => {
            tracing::debug!("Headers fetched successfully.");
            let results = join!(
                calculate_twap(twap),
                calculate_volatility(volatility),
                calculate_reserve_price(reserve)
            );
            match results {
                (Ok(twap), Ok(volatility), Ok(reserve_price)) => {
                    Some((twap, volatility, reserve_price))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

// Validate the provided time ranges
fn validate_time_ranges(
    params: &PitchLakeJobRequestParams,
) -> Result<(), (StatusCode, JobResponse)> {
    let validations = [
        ("TWAP", params.twap),
        ("Volatility", params.volatility),
        ("Reserve Price", params.reserve_price),
    ];

    for (name, (start, end)) in &validations {
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
        assert_eq!(response.message, "Processing initiated.");
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
        assert_eq!(response.message, "Job is already pending.");
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
            "Job completed. Fetch results from the status endpoint."
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
        assert_eq!(response.message, "Reprocessing initiated.");
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
        assert_eq!(response.message, "Processing initiated.");
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

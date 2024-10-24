use std::sync::Arc;

use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use starknet_crypto::{poseidon_hash_single, Felt};
use starknet_handler::{FossilStarknetAccount, JobRequest, PitchLakeResult, PITCH_LAKE_V1};
use tokio::{join, runtime::Handle, time::Instant};

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
use starknet::core::types::U256;

// Main handler function
pub async fn get_pricing_data(
    State(state): State<AppState>,
    Json(payload): Json<PitchLakeJobRequest>,
) -> (StatusCode, Json<JobResponse>) {
    tracing::info!("Received pricing data request.");

    if let Err((status, response)) = validate_request(&payload) {
        tracing::warn!("Invalid request: {:?}", response);
        return (status, Json(response));
    }

    let starknet_account = FossilStarknetAccount::new();
    let job_id = generate_job_id(&payload.identifiers, &payload.params);

    tracing::info!("Generated job_id: {}", job_id);

    match get_job_request(&state.db.pool, &job_id).await {
        Ok(Some(job_request)) => {
            handle_existing_job(
                &state,
                job_request.status,
                job_id,
                payload,
                starknet_account,
            )
            .await
        }
        Ok(None) => {
            tracing::info!("Creating new job request.");
            handle_new_job_request(&state, job_id, payload, starknet_account).await
        }
        Err(e) => internal_server_error(e, job_id),
    }
}

// Helper to validate the request
fn validate_request(payload: &PitchLakeJobRequest) -> Result<(), (StatusCode, JobResponse)> {
    if payload.identifiers.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            JobResponse::new(
                String::new(),
                Some("Identifiers cannot be empty.".to_string()),
                None,
            ),
        ));
    }
    validate_time_ranges(&payload.params)
}

// Helper to generate a job ID
fn generate_job_id(identifiers: &[String], params: &PitchLakeJobRequestParams) -> String {
    let mut input = identifiers.join("");

    // Concatenate all time ranges as part of the job ID generation
    input.push_str(&format!(
        "{}{}{}{}{}{}",
        params.twap.0,
        params.twap.1,
        params.volatility.0,
        params.volatility.1,
        params.reserve_price.0,
        params.reserve_price.1
    ));

    // Hash the concatenated string using Poseidon
    poseidon_hash_single(Felt::from_bytes_be_slice(input.as_bytes())).to_string()
}

// Handle existing jobs based on status
async fn handle_existing_job(
    state: &AppState,
    status: JobStatus,
    job_id: String,
    payload: PitchLakeJobRequest,
    starknet_account: FossilStarknetAccount,
) -> (StatusCode, Json<JobResponse>) {
    match status {
        JobStatus::Pending => job_response(
            StatusCode::CONFLICT, // 409 Conflict
            job_id,
            "Job is already pending. Use the status endpoint to monitor progress.",
        ),
        JobStatus::Completed => job_response(
            StatusCode::OK, // 200 OK
            job_id,
            "Job has already been completed. No further processing required.",
        ),
        JobStatus::Failed => reprocess_failed_job(state, job_id, payload, starknet_account).await,
    }
}

// Handle new job requests
async fn handle_new_job_request(
    state: &AppState,
    job_id: String,
    payload: PitchLakeJobRequest,
    starknet_account: FossilStarknetAccount,
) -> (StatusCode, Json<JobResponse>) {
    match create_job_request(&state.db.pool, &job_id, JobStatus::Pending).await {
        Ok(_) => {
            tracing::info!("New job request registered and processing initiated.");
            let db_clone = state.db.clone();
            let job_id_clone = job_id.clone();
            let handle = Handle::current();

            tokio::task::spawn_blocking(move || {
                handle.block_on(process_job(
                    db_clone,
                    job_id_clone,
                    payload,
                    starknet_account,
                ));
            });

            (
                StatusCode::CREATED,
                Json(JobResponse {
                    job_id: job_id.clone(),
                    message: Some(
                        "New job request registered and processing initiated.".to_string(),
                    ),
                    status: Some(JobStatus::Pending),
                }),
            )
        }
        Err(e) => internal_server_error(e, job_id),
    }
}

// Helper to handle failed job reprocessing
async fn reprocess_failed_job(
    state: &AppState,
    job_id: String,
    payload: PitchLakeJobRequest,
    starknet_account: FossilStarknetAccount,
) -> (StatusCode, Json<JobResponse>) {
    if let Err(e) = update_job_status(&state.db.pool, &job_id, JobStatus::Pending, None).await {
        return internal_server_error(e, job_id);
    }
    let db_clone = state.db.clone();
    let job_id_clone = job_id.clone();
    let handle = Handle::current();

    tokio::task::spawn_blocking(move || {
        handle.block_on(process_job(
            db_clone,
            job_id_clone,
            payload,
            starknet_account,
        ));
    });

    job_response(
        StatusCode::OK, // Ensure it's 200 OK
        job_id,
        "Previous job request failed. Reprocessing initiated.",
    )
}

// Helper to generate a JSON response
fn job_response(
    status: StatusCode,
    job_id: String,
    message: &str,
) -> (StatusCode, Json<JobResponse>) {
    tracing::info!("Responding to job {} with status {}", job_id, status);
    (
        status,
        Json(JobResponse::new(job_id, Some(message.to_string()), None)),
    )
}

// Handle internal server errors
fn internal_server_error(error: sqlx::Error, job_id: String) -> (StatusCode, Json<JobResponse>) {
    tracing::error!("Internal server error: {:?}", error);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(JobResponse::new(
            job_id,
            Some(format!("An error occurred: {}", error)),
            None,
        )),
    )
}

// Process the job and trigger the Starknet callback
async fn process_job(
    db: Arc<DbConnection>,
    job_id: String,
    payload: PitchLakeJobRequest,
    starknet_account: FossilStarknetAccount,
) {
    tracing::info!("Starting job {} processing.", job_id);
    tracing::debug!("Payload received: {:?}", payload);

    match fetch_headers(&db, &payload).await {
        Some((twap, volatility, reserve_price)) => {
            tracing::info!(
                "Fetched block headers for job {}. Calculated values: TWAP = {}, Volatility = {}, Reserve Price = {}",
                job_id, twap, volatility, reserve_price
            );

            let result = PitchLakeResult {
                twap: U256::from(twap as u128),
                volatility: volatility as u128,
                reserve_price: U256::from(reserve_price as u128),
            };

            if let Err(e) = update_job_status(
                &db.pool,
                &job_id,
                JobStatus::Completed,
                Some(serde_json::json!({
                    "twap": twap,
                    "volatility": volatility,
                    "reserve_price": reserve_price,
                })),
            )
            .await
            {
                tracing::error!("Failed to update job status for {}: {:?}", job_id, e);
                return;
            }

            tracing::info!(
                "Job {} completed. Initiating Starknet callback to contract at address: {}",
                job_id,
                payload.client_info.client_address
            );

            let job_request = JobRequest {
                vault_address: payload.client_info.vault_address,
                timestamp: payload.client_info.timestamp,
                program_id: Felt::from_hex(PITCH_LAKE_V1).unwrap(),
            };

            tracing::debug!(
                "Starknet callback calldata: Client Address = {:?}, Vault Address = {:?}, Timestamp = {}, Program ID = {}",
                job_request.vault_address,
                payload.client_info.vault_address,
                job_request.timestamp,
                PITCH_LAKE_V1
            );

            match starknet_account
                .callback_to_contract(payload.client_info.client_address, &job_request, &result)
                .await
            {
                Ok(tx_hash) => {
                    tracing::info!(
                        "Starknet callback successful for job {}. Transaction hash: {}",
                        job_id,
                        tx_hash
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Starknet callback failed for job {}. Error: {:?}",
                        job_id,
                        e
                    );
                    let _ = update_job_status(&db.pool, &job_id, JobStatus::Failed, None).await;
                }
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
            
            let now = Instant::now();
            tracing::info!("Started processing...");
            
            let results = join!(
                calculate_twap(twap),
                calculate_volatility(volatility),
                calculate_reserve_price(reserve)
            );

            let elapsed = now.elapsed();
            tracing::info!("Elapsed: {:.2?}", elapsed);

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
                JobResponse::new(
                    String::new(),
                    Some(format!("Invalid time range for {} calculation.", name)),
                    None,
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::fixtures::TestContext;
    use crate::types::{ClientInfo, PitchLakeJobRequest, PitchLakeJobRequestParams};
    use axum::http::StatusCode;
    use starknet::core::types::Felt;

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
            client_info: ClientInfo {
                client_address: Felt::from_hex("0x123").unwrap(),
                vault_address: Felt::from_hex("0x456").unwrap(),
                timestamp: 0,
            },
        };

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::CREATED);
        assert!(!response.job_id.is_empty());
        assert_eq!(
            response.message.unwrap(),
            "New job request registered and processing initiated."
        );
    }

    #[tokio::test]
    async fn test_get_pricing_data_pending_job() {
        let ctx = TestContext::new().await;

        let payload = PitchLakeJobRequest {
            identifiers: vec!["test-id".to_string()],
            params: PitchLakeJobRequestParams {
                twap: (0, 100),
                volatility: (0, 100),
                reserve_price: (0, 100),
            },
            client_info: ClientInfo {
                client_address: Felt::from_hex("0x123").unwrap(),
                vault_address: Felt::from_hex("0x456").unwrap(),
                timestamp: 0,
            },
        };

        let job_id = generate_job_id(&payload.identifiers, &payload.params);
        ctx.create_job(&job_id, JobStatus::Pending).await;

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(response.job_id, job_id);
        assert_eq!(
            response.message.unwrap_or_default(),
            "Job is already pending. Use the status endpoint to monitor progress."
        );
    }

    #[tokio::test]
    async fn test_get_pricing_data_completed_job() {
        let ctx = TestContext::new().await;

        let payload = PitchLakeJobRequest {
            identifiers: vec!["test-id".to_string()],
            params: PitchLakeJobRequestParams {
                twap: (0, 100),
                volatility: (0, 100),
                reserve_price: (0, 100),
            },
            client_info: ClientInfo {
                client_address: Felt::from_hex("0x123").unwrap(),
                vault_address: Felt::from_hex("0x456").unwrap(),
                timestamp: 0,
            },
        };

        let job_id = generate_job_id(&payload.identifiers, &payload.params);
        ctx.create_job(&job_id, JobStatus::Completed).await;

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.job_id, job_id);
        assert_eq!(
            response.message.unwrap_or_default(),
            "Job has already been completed. No further processing required."
        );
    }

    #[tokio::test]
    async fn test_get_pricing_data_failed_job() {
        let ctx = TestContext::new().await;

        let payload = PitchLakeJobRequest {
            identifiers: vec!["test-id".to_string()],
            params: PitchLakeJobRequestParams {
                twap: (0, 100),
                volatility: (0, 100),
                reserve_price: (0, 100),
            },
            client_info: ClientInfo {
                client_address: Felt::from_hex("0x123").unwrap(),
                vault_address: Felt::from_hex("0x456").unwrap(),
                timestamp: 0,
            },
        };

        let job_id = generate_job_id(&payload.identifiers, &payload.params);
        ctx.create_job(&job_id, JobStatus::Failed).await;

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.job_id, job_id);
        assert_eq!(
            response.message.unwrap_or_default(),
            "Previous job request failed. Reprocessing initiated."
        );
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
            client_info: ClientInfo {
                client_address: Felt::from_hex("0x123").unwrap(),
                vault_address: Felt::from_hex("0x456").unwrap(),
                timestamp: 0,
            },
        };

        let (status, Json(response)) = ctx.get_pricing_data(payload).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            response.message.unwrap_or_default(),
            "Invalid time range for TWAP calculation."
        );
    }
}

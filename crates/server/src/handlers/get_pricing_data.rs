use dotenv::dotenv;
use std::env;
use std::sync::Arc;

use crate::types::{JobResponse, PitchLakeJobRequest};
use crate::AppState;
use crate::{
    pricing_data::{
        cap_level::calculate_cap_level, reserve_price::calculate_reserve_price,
        twap::calculate_twap,
    },
    types::PitchLakeJobRequestParams,
};
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
use eyre::{eyre, Result};
use starknet::core::types::U256;
use starknet_crypto::{poseidon_hash_single, Felt};
use starknet_handler::{FossilStarknetAccount, JobRequest, PitchLakeResult, PITCH_LAKE_V1};
use tokio::{join, runtime::Handle, time::Instant};

// Main handler function
pub async fn get_pricing_data(
    State(state): State<AppState>,
    Json(payload): Json<PitchLakeJobRequest>,
) -> (StatusCode, Json<JobResponse>) {
    let identifiers = payload.identifiers.join(",");
    let context = format!(
        "identifiers=[{}], timestamp={}, twap-range=({},{}), cap_level-range=({},{}), reserve_price-range=({},{}), client_address={:#064x}, vault_address={:#064x}",
        identifiers,
        payload.client_info.timestamp,
        payload.params.twap.0, payload.params.twap.1,
        payload.params.cap_level.0, payload.params.cap_level.1,
        payload.params.reserve_price.0, payload.params.reserve_price.1,
        payload.client_info.client_address,
        payload.client_info.vault_address,
    );

    tracing::info!("Received pricing data request. {}", context);

    if let Err((status, response)) = validate_request(&payload) {
        tracing::warn!("Invalid request: {:?}. {}", response, context);
        return (status, Json(response));
    }

    let starknet_account = FossilStarknetAccount::default();
    let job_id = generate_job_id(&payload.identifiers, &payload.params);

    tracing::info!("Generated job_id: {}. {}", job_id, context);

    match get_job_request(&state.db.pool, &job_id).await {
        Ok(Some(job_request)) => {
            tracing::info!(
                "Found existing job with status: {}. {}",
                job_request.status,
                context
            );
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
            tracing::info!("Creating new job request. {}", context);
            handle_new_job_request(&state, job_id, payload, starknet_account).await
        }
        Err(e) => {
            tracing::error!("Database error: {}. {}", e, context);
            internal_server_error(e, job_id)
        }
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
        "{}{}{}{}{}{}{}{}",
        params.twap.0,
        params.twap.1,
        params.cap_level.0,
        params.cap_level.1,
        params.reserve_price.0,
        params.reserve_price.1,
        params.alpha,
        params.k,
    ));

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
            StatusCode::CONFLICT,
            job_id,
            "Job is already pending. Use the status endpoint to monitor progress.",
        ),
        JobStatus::Completed => job_response(
            StatusCode::OK,
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
        StatusCode::OK,
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
    let context = format!(
        "job_id={}, identifiers=[{}], twap=({},{}), cap_level=({},{}), reserve_price=({},{}), alpha={}, k={}, client_address={:#064x}, vault_address={:#064x}",
        job_id,
        payload.identifiers.join(","),
        payload.params.twap.0, payload.params.twap.1,
        payload.params.cap_level.0, payload.params.cap_level.1,
        payload.params.reserve_price.0, payload.params.reserve_price.1,
        payload.params.alpha,
        payload.params.k,
        payload.client_info.client_address,
        payload.client_info.vault_address,
    );

    tracing::info!("Starting job processing. {}", context);
    tracing::debug!("Payload received: {:?}. {}", payload, context);

    let job_result = match fetch_headers(&db, &payload).await {
        Ok(Some((twap, cap_level, reserve_price))) => {
            tracing::info!(
                "Fetched block headers. Calculated values: TWAP = {}, Cap Level = {}, Reserve Price = {}. {}",
                twap, cap_level, reserve_price, context
            );

            let result = PitchLakeResult {
                twap: U256::from(twap as u128),
                cap_level: (cap_level * 10_000.0) as u128,
                reserve_price: U256::from(reserve_price as u128),
            };

            if let Err(e) = update_job_status(
                &db.pool,
                &job_id,
                JobStatus::Completed,
                Some(serde_json::json!({
                    "twap": twap,
                    "cap_level": cap_level,
                    "reserve_price": reserve_price,
                })),
            )
            .await
            {
                tracing::error!("Failed to update job status: {:?}. {}", e, context);
                return;
            }

            tracing::info!(
                "Job completed. Initiating Starknet callback to contract at address: {}. {}",
                payload.client_info.client_address,
                context
            );

            let program_id = match Felt::from_hex(PITCH_LAKE_V1) {
                Ok(id) => id,
                Err(e) => {
                    let error_msg = format!("Failed to parse program ID: {:?}", e);
                    tracing::error!("{}. {}", error_msg, context);
                    let _ = update_job_status(
                        &db.pool,
                        &job_id,
                        JobStatus::Failed,
                        Some(serde_json::json!({
                            "error": error_msg
                        })),
                    )
                    .await;
                    return;
                }
            };

            let job_request = JobRequest {
                vault_address: payload.client_info.vault_address,
                timestamp: payload.client_info.timestamp.to_string(),
                program_id,
                alpha: payload.params.alpha,
                k: payload.params.k,
            };

            tracing::debug!(
                "Starknet callback calldata: Client Address = {:?}, Vault Address = {:?}, Timestamp = {}, Program ID = {}. {}",
                payload.client_info.client_address,
                payload.client_info.vault_address,
                job_request.timestamp,
                PITCH_LAKE_V1,
                context
            );

            match starknet_account
                .callback_to_contract(payload.client_info.client_address, &job_request, &result)
                .await
            {
                Ok(_) => {
                    tracing::info!("Job processing finished successfully. {}", context);
                    true
                }
                Err(e) => {
                    let error_msg = format!("Starknet callback failed. Error: {:?}", e);
                    tracing::error!("{}. {}", error_msg, context);
                    let _ = update_job_status(
                        &db.pool,
                        &job_id,
                        JobStatus::Failed,
                        Some(serde_json::json!({
                            "error": error_msg
                        })),
                    )
                    .await;
                    false
                }
            }
        }
        Ok(None) => {
            let error_msg = "Failed to fetch headers or calculate pricing data";
            tracing::error!("{}. {}", error_msg, context);
            let _ = update_job_status(
                &db.pool,
                &job_id,
                JobStatus::Failed,
                Some(serde_json::json!({
                    "error": error_msg
                })),
            )
            .await;
            false
        }
        Err(e) => {
            let error_msg = format!("Error fetching headers: {:?}", e);
            tracing::error!("{}. {}", error_msg, context);
            let _ = update_job_status(
                &db.pool,
                &job_id,
                JobStatus::Failed,
                Some(serde_json::json!({
                    "error": error_msg
                })),
            )
            .await;
            false
        }
    };

    if job_result {
        tracing::info!("Job processing finished successfully. {}", context);
    } else {
        tracing::error!(
            "Job processing failed. See previous errors for details. {}",
            context
        );
    }
}

// Helper to fetch block headers in parallel
async fn fetch_headers(
    db: &Arc<DbConnection>,
    payload: &PitchLakeJobRequest,
) -> Result<Option<(f64, f64, f64)>, eyre::Error> {
    tracing::debug!("Fetching block headers for calculations.");

    dotenv().ok();
    let use_mock_pricing_data = env::var("USE_MOCK_PRICING_DATA")
        .map_err(|_| eyre!("USE_MOCK_PRICING_DATA should be provided as env vars."))?;

    if use_mock_pricing_data.to_lowercase() == "true" {
        tracing::info!("Using mock pricing data");
        return Ok(Some((14732102267.474916, 440.0, 2597499408.638207)));
    }

    let (twap_headers, cap_level_headers, reserve_price_headers) = join!(
        get_block_headers_by_time_range(
            &db.pool,
            payload.params.twap.0.to_string(),
            payload.params.twap.1.to_string()
        ),
        get_block_headers_by_time_range(
            &db.pool,
            payload.params.cap_level.0.to_string(),
            payload.params.cap_level.1.to_string()
        ),
        get_block_headers_by_time_range(
            &db.pool,
            payload.params.reserve_price.0.to_string(),
            payload.params.reserve_price.1.to_string()
        )
    );

    let alpha = payload.params.alpha;
    let k = payload.params.k;

    match (twap_headers, cap_level_headers, reserve_price_headers) {
        (Ok(twap), Ok(cap_level), Ok(reserve)) => {
            tracing::debug!("Block headers fetched successfully.");

            let now = Instant::now();
            tracing::info!("Started processing...");

            // Get twap future
            let twap = calculate_twap(twap);

            // Get cap level future
            let cap_level = calculate_cap_level(alpha, k, cap_level);
            // Get cap level value
            let cap_level_result = cap_level.await;
            // Get reserve price future
            let reserve_price = match cap_level_result {
                Ok(val) => calculate_reserve_price(reserve, val), // val is f64 and Copy
                Err(e) => {
                    tracing::error!("No cap level to pass to reserve price {}.", e);
                    return Err(e);
                }
            };

            // Convert cap level back into a future
            let cap_level = async { cap_level_result }; // pseudo-future to satisfy `join!`

            let results = join!(twap, cap_level, reserve_price);

            let elapsed = now.elapsed();
            tracing::info!("Elapsed: {:.2?}", elapsed);

            match results {
                (Ok(twap), Ok(cap_level), Ok(reserve_price)) => {
                    Ok(Some((twap, cap_level, reserve_price)))
                }
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

// Validate the provided time ranges
fn validate_time_ranges(
    params: &PitchLakeJobRequestParams,
) -> Result<(), (StatusCode, JobResponse)> {
    let validations = [
        ("TWAP", params.twap),
        ("Cap Level", params.cap_level),
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
                cap_level: (0, 100),
                reserve_price: (0, 100),
                alpha: 2500,
                k: 0,
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
                cap_level: (0, 100),
                reserve_price: (0, 100),
                alpha: 2500,
                k: 0,
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
                cap_level: (0, 100),
                reserve_price: (0, 100),
                alpha: 2500,
                k: 0,
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
                cap_level: (0, 100),
                reserve_price: (0, 100),
                alpha: 2500,
                k: 0,
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
                cap_level: (0, 100),
                reserve_price: (0, 100),
                alpha: 2500,
                k: 0,
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

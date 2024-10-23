use crate::types::{ErrorResponse, GetJobStatusResponseEnum, JobResponse};
use crate::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use db_access::queries::get_job_request;

#[axum::debug_handler]
pub async fn get_job_status(
    State(state): State<AppState>, // Use AppState as the state
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> (StatusCode, Json<GetJobStatusResponseEnum>) {
    match get_job_request(&state.db.pool, &job_id).await {
        Ok(Some(job)) => (
            StatusCode::OK,
            Json(GetJobStatusResponseEnum::Success(JobResponse {
                job_id: job.job_id,
                message: None,
                status: Some(job.status),
            })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(GetJobStatusResponseEnum::Error(ErrorResponse {
                error: "Job not found".to_string(),
            })),
        ),
        Err(e) => {
            tracing::error!("Failed to get job status: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GetJobStatusResponseEnum::Error(ErrorResponse {
                    error: "An internal error occurred. Please try again later.".to_string(),
                })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use core::panic;

    use crate::handlers::fixtures::TestContext;
    use axum::http::StatusCode;
    use db_access::models::JobStatus;
    use serde_json::json;

    #[tokio::test]
    async fn test_get_job_status_not_found() {
        let ctx = TestContext::new().await;
        let job_id = "non_existent_job_id";

        let (status, Json(response)) = ctx.get_job_status(job_id).await;

        let response = match response {
            GetJobStatusResponseEnum::Error(err_response) => err_response,
            GetJobStatusResponseEnum::Success(_) => panic!("Unexpected response status"),
        };

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(response.error, "Job not found");
    }

    #[tokio::test]
    async fn test_get_job_status_pending() {
        let ctx = TestContext::new().await;
        let job_id = "pending_job_id";

        ctx.create_job(job_id, JobStatus::Pending).await;

        let (status, Json(response)) = ctx.get_job_status(job_id).await;

        let response = match response {
            GetJobStatusResponseEnum::Success(success_res) => success_res,
            GetJobStatusResponseEnum::Error(_) => panic!("Unexpected response status"),
        };

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.job_id, job_id);
        assert_eq!(response.status.unwrap(), JobStatus::Pending);
    }

    #[tokio::test]
    async fn test_get_job_status_failed() {
        let ctx = TestContext::new().await;
        let job_id = "failed_job_id";

        ctx.create_job(job_id, JobStatus::Failed).await;

        let (status, Json(response)) = ctx.get_job_status(job_id).await;

        let response = match response {
            GetJobStatusResponseEnum::Success(success_res) => success_res,
            GetJobStatusResponseEnum::Error(_) => panic!("Unexpected response status"),
        };

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.job_id, job_id);
        assert_eq!(response.status.unwrap(), JobStatus::Failed);
    }

    #[tokio::test]
    async fn test_get_job_status_completed() {
        let ctx = TestContext::new().await;
        let job_id = "completed_job_id";

        // Create a completed job with a sample result
        let sample_result = json!({
            "twap": 12345.67,
            "volatility": 2345.89,
            "reserve_price": 3456.78
        });

        ctx.create_job_with_result(job_id, JobStatus::Completed, sample_result.clone())
            .await;

        let (status, Json(response)) = ctx.get_job_status(job_id).await;

        let response = match response {
            GetJobStatusResponseEnum::Success(success_res) => success_res,
            GetJobStatusResponseEnum::Error(_) => panic!("Unexpected response status"),
        };

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.job_id, job_id);
        assert_eq!(response.status.unwrap(), JobStatus::Completed);
    }
}

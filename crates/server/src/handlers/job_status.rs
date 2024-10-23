use crate::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use db_access::queries::get_job_request;
use serde_json::json;
// use chrono::{DateTime, Utc};

#[axum::debug_handler]
pub async fn get_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    match get_job_request(&state.db.pool, &job_id).await {
        Ok(Some(job)) => (
            StatusCode::OK,
            Json(json!({
                "job_id": job.job_id,
                "status": job.status.to_string(),
                "created_at": job.created_at.and_utc(), // Convert to UTC if it's NaiveDateTime
            })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Job not found".to_string(),
            })),
        ),
        Err(e) => {
            tracing::error!("Failed to get job status: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "An internal error occurred. Please try again later.".to_string(),
                })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::handlers::fixtures::TestContext;

    use super::*;
    use axum::http::StatusCode;
    use db_access::models::JobStatus;

    #[tokio::test]
    async fn test_get_job_status_not_found() {
        let ctx = TestContext::new().await;
        let job_id = "non_existent_job_id";
        let (status, Json(response)) = ctx.get_job_status(job_id).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(response["error"], "Job not found");
    }

    #[tokio::test]
    async fn test_get_job_status_pending() {
        let ctx = TestContext::new().await;
        let job_id = "pending_job_id";

        ctx.create_job(job_id, JobStatus::Pending).await;

        let (status, Json(response)) = ctx.get_job_status(job_id).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response["job_id"], job_id);
        assert_eq!(response["status"], "Pending");
    }

    #[tokio::test]
    async fn test_get_job_status_failed() {
        let ctx = TestContext::new().await;
        let job_id = "failed_job_id";

        ctx.create_job(job_id, JobStatus::Failed).await;

        let (status, Json(response)) = ctx.get_job_status(job_id).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response["job_id"], job_id);
        assert_eq!(response["status"], "Failed");
    }

    #[tokio::test]
    async fn test_get_job_status_completed() {
        let ctx = TestContext::new().await;
        let job_id = "completed_job_id";

        ctx.create_job(job_id, JobStatus::Completed).await;

        let (status, Json(response)) = ctx.get_job_status(job_id).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response["job_id"], job_id);
        assert_eq!(response["status"], "Completed");
    }
}

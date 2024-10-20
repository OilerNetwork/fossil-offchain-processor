use crate::types::{ErrorResponse, JobStatusResponse};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use db_access::{queries::get_job_request, DbConnection};
use serde_json::json;
use tracing::error;

pub async fn get_job_status(
    State(db): State<DbConnection>,
    Path(job_id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    match get_job_request(&db.pool, &job_id).await {
        Ok(Some(job)) => (
            StatusCode::OK,
            Json(json!(JobStatusResponse {
                job_id: job.job_id,
                status: job.status.as_str().to_string(),
            })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!(ErrorResponse {
                error: "Job not found".to_string(),
            })),
        ),
        Err(e) => {
            error!("Failed to get job status: {:?}", e);

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(ErrorResponse {
                    error: "An internal error occurred. Please try again later.".to_string(),
                })),
            )
        }
    }
}

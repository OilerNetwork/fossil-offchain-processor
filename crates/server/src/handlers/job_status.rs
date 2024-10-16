use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use db_access::{DbConnection, queries::get_job_request};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
pub struct JobStatusResponse {
    job_id: String,
    status: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

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
            }))
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!(ErrorResponse {
                error: "Job not found".to_string(),
            }))
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!(ErrorResponse {
                error: "An error occurred while processing the request.".to_string(),
            }))
        ),
    }
}

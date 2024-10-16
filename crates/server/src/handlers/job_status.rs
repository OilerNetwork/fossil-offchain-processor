use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use db_access::{DbConnection, queries::get_job_request};
use serde::Serialize;

#[derive(Serialize)]
pub struct JobStatusResponse {
    job_id: String,
    status: String,
}

pub async fn get_job_status(
    State(db): State<DbConnection>,
    Path(job_id): Path<String>,
) -> Result<Json<JobStatusResponse>, (StatusCode, String)> {
    match get_job_request(&db.pool, &job_id).await {
        Ok(Some(job)) => Ok(Json(JobStatusResponse {
            job_id: job.job_id,
            status: job.status.as_str().to_string(),
        })),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Job not found".to_string())),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())),
    }
}

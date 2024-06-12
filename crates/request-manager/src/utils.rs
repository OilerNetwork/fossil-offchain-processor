use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

pub fn response_with_status(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "message": message }))).into_response()
}

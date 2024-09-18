use axum::{http::StatusCode, Json};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct JobRequest {
    identifier: Vec<String>,
    timestamp: u128,
    params: Vec<String>,
}

// We can keep this as a simple "Hello, world!" for now
// but its a good place to place a health check endpoint
pub async fn root() -> &'static str {
    "Hello, world!"
}

pub async fn get_twap(Json(payload): Json<JobRequest>) -> (StatusCode, &'static str) {
    (StatusCode::OK, "twap")
}

pub async fn get_volatility(Json(payload): Json<JobRequest>) -> (StatusCode, &'static str) {
    (StatusCode::OK, "volatility")
}

pub async fn get_reserve_price(Json(payload): Json<JobRequest>) -> (StatusCode, &'static str) {
    (StatusCode::OK, "reserve_price")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, routing::get, Router};

    use axum_test::TestServer;

    #[tokio::test]
    async fn test_root() {
        let app = Router::new().route("/", get(root));
        let server = TestServer::new(app).unwrap();

        let response = server.get("/").await;

        assert_eq!(response.status_code(), StatusCode::OK);
        assert_eq!(response.text(), "Hello, world!");
    }
}

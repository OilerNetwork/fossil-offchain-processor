use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};

// timestamp ranges for each sub-job calculation 
#[derive(Debug, Deserialize, Serialize)]
pub struct PitchLakeJobRequestParams {
    twap: (u64, u64),
    volatility: (u64, u64),
    reserve_price: (u64, u64),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PitchLakeJobRequest {
    identifiers: Vec<String>,
    params: PitchLakeJobRequestParams,
}

// We can keep this as a simple "Hello, world!" for now
// but its a good place to place a health check endpoint
pub async fn root() -> &'static str {
    "Hello, world!"
}

pub async fn get_pricing_data(Json(payload): Json<PitchLakeJobRequest>) -> (StatusCode, &'static str) {
    (StatusCode::OK, "pricing_data")
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

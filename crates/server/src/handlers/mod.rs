use std::collections::HashMap;

use axum::{extract::State, http::StatusCode, Json};
use db_access::DbConnection;
use serde::{Deserialize, Serialize};
use twap::calculate_twap;
use reserve_price::calculate_reserve_price;

pub mod twap;
pub mod reserve_price;
// timestamp ranges for each sub-job calculation
#[derive(Debug, Deserialize, Serialize)]
pub struct PitchLakeJobRequestParams {
    twap: (i64, i64),
    volatility: (i64, i64),
    reserve_price: (i64, i64),
    callback_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PitchLakeJobRequest {
    identifiers: Vec<String>,
    params: PitchLakeJobRequestParams,
}

// TODO: Placeholder for now, need to be more generic
#[derive(Debug, Deserialize, Serialize)]
pub struct PitchLakeJobCallback {
    job_id: String,
    twap: HashMap<String, i64>,
    volatility: i64,
    reserve_price: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JobResponse {
    job_id: String,
}

// We can keep this as a simple "Hello, world!" for now
// but its a good place to place a health check endpoint
pub async fn root() -> &'static str {
    "OK"
}

pub async fn get_pricing_data(
    State(db): State<DbConnection>,
    Json(payload): Json<PitchLakeJobRequest>,
) -> (StatusCode, Json<JobResponse>) {
    tokio::spawn(async move {
        let twap = calculate_twap(&db, payload.params.twap.0, payload.params.twap.1).await;
        println!("twap: {:?}", twap);
    });

    // TODO(cwk): save the jobid somewhere
    (
        StatusCode::OK,
        Json(JobResponse {
            job_id: "123".to_string(),
        }),
    )
}

pub async fn get_reserve_price(
    Json(payload): Json<PitchLakeJobRequest>,
) -> (StatusCode, Json<JobResponse>) {
    let (start_timestamp, end_timestamp): (i64, i64) = payload.params.reserve_price;
    println!("start_timestamp: {:?}, end_timestamp: {:?}", start_timestamp, end_timestamp);
    let job_id = "456";

    tokio::spawn(async move {
        match calculate_reserve_price(start_timestamp, end_timestamp).await {
            Ok(reserve_price) => println!("Job {}: Reserve price calculated: {:?}", job_id, reserve_price),
            Err(e) => eprintln!("Job {}: Failed to calculate reserve price: {:?}", job_id, e),
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(JobResponse {
            job_id: job_id.to_string(),
        }),
    )
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
        assert_eq!(response.text(), "OK");
    }
}

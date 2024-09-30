use axum::{extract::State, http::StatusCode, Json};
use db_access::DbConnection;
use serde::{Deserialize, Serialize};
use twap::calculate_twap;
use reserve_price::calculate_reserve_price;
use volatility::calculate_volatility;
use starknet_crypto::{poseidon_hash_single, Felt};
use std::collections::HashMap;

mod twap;
mod utils;
mod volatility;
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

pub async fn root() -> &'static str {
    "OK"
}

pub async fn get_pricing_data(
    State(db): State<DbConnection>,
    Json(payload): Json<PitchLakeJobRequest>,
) -> (StatusCode, Json<JobResponse>) {
    let job_id = poseidon_hash_single(Felt::from_bytes_be_slice(
        payload.identifiers.join("").as_bytes(),
    ));
    // TODO(cwk): save the jobid somewhere

    tokio::spawn(async move {
        let twap = calculate_twap(&db, payload.params.twap.0, payload.params.twap.1).await;
        let volatility_result: Result<u128, anyhow::Error> = calculate_volatility(
            &db,
            payload.params.volatility.0,
            payload.params.volatility.1,
        )
        .await;

        let reserve_price = calculate_reserve_price(
            payload.params.reserve_price.0,
            payload.params.reserve_price.1,
        )
        .await;

        println!("twap: {:?}", twap);
        println!("volatility_result: {:?}", volatility_result);
        println!("reserve_price: {:?}", reserve_price);
    });

    (
        StatusCode::OK,
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

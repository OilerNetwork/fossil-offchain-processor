use axum::{http::StatusCode, Json};

use super::get_pricing_data::PitchLakeJobCallback;

/// This callback is a placeholder for testing purposes only.
pub async fn pricing_callback(
    Json(payload): Json<PitchLakeJobCallback>,
) -> (StatusCode, &'static str) {
    println!("Payload received! {:?}", payload);

    (StatusCode::OK, "Callback OK!")
}

// No test needed here, since we only use this for internal testing purposes. Due for removal.

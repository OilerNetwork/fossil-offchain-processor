use tracing::info;

pub async fn pricing_callback(
    Json(payload): Json<PitchLakeJobCallback>,
) -> (StatusCode, &'static str) {
    info!("Payload received: {:?}", payload);

    (StatusCode::OK, "Callback OK!")
}

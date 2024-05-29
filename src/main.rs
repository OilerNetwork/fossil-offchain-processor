mod controller;
mod model;
use std::str::FromStr;

use axum::{
    extract::{FromRef, MatchedPath},
    http::Request,
    routing::post,
    Router,
};
use fossil_offchain_processor::relayer::contract::TestContract;
use reqwest::Client;

use dotenv::dotenv;
use secp256k1::SecretKey;
use tower_http::trace::TraceLayer;
use tracing::info_span;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::controller::calculate_get_storage::call_mev_blocker_api;

#[derive(Clone)]
struct AppState {
    client: Client,
}

// support converting an `AppState` in an `ApiState`
impl FromRef<AppState> for Client {
    fn from_ref(app_state: &AppState) -> Client {
        app_state.client.clone()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "fossil_offchain_processor=info,tower_http=debug,axum=info,tokio=info".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    dotenv().ok();
    let app_state = AppState {
        client: Client::new(),
    };

    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", post(call_mev_blocker_api))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                // Log the matched route's path (with placeholders not filled in).
                // Use request.uri() or OriginalUri if you want the real path.
                let matched_path = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map(MatchedPath::as_str);

                info_span!(
                    "http_request",
                    method = ?request.method(),
                    matched_path,
                    some_other_field = tracing::field::Empty,
                )
            }),
        )
        .with_state(app_state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    let transport = web3::transports::Http::new("http://127.0.0.1:8545").unwrap();
    let web3 = web3::Web3::new(transport);

    let test_contract = TestContract::new(
        &web3,
        "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string(),
    )
    .await;

    test_contract.send_latest_parent_has_to_l2().await;

    tracing::info!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

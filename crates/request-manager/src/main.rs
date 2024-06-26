use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{FromRef, MatchedPath},
    http::Request,
    routing::post,
    Router,
};
use reqwest::Client;

use dotenv::dotenv;
use serde::Deserialize;
use tower_http::trace::TraceLayer;
use tracing::info_span;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use proof_generator::controller::mev_blocker::call_mev_blocker_api;
use utils::get_storage_value;

pub mod utils;

#[derive(Clone)]
struct AppState {
    client: Client,
    storage_cache: Arc<Mutex<HashMap<String, String>>>,
}

// support converting an `AppState` in an `ApiState`
impl FromRef<AppState> for Client {
    fn from_ref(app_state: &AppState) -> Client {
        app_state.client.clone()
    }
}

#[derive(Deserialize)]
struct StorageRequest {
    account_address: String,
    storage_key: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "request_manager=info,tower_http=debug,axum=info,tokio=info".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    dotenv().ok();

    let app_state = AppState {
        client: Client::new(),
        storage_cache: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", post(call_mev_blocker_api))
        .route("/getStorageValue", post(get_storage_value))
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

    // run our app with hyper, listening globally on port 8000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    tracing::info!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

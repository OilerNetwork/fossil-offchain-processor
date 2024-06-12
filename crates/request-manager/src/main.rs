use axum::{
    extract::{FromRef, MatchedPath},
    http::{Request, StatusCode},
    routing::post,
    Router,
};
use dotenv::dotenv;
use reqwest::Client;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;
use tracing::info_span;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod handlers;
mod state;
mod utils;

use handlers::get_storage_value::get_storage_value;
use state::AppState;

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
        .route("/getStorageValue", post(get_storage_value))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
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

    let listener = match tokio::net::TcpListener::bind("0.0.0.0:8000").await {
        Ok(listener) => {
            tracing::info!("Listening on http://{}", listener.local_addr().unwrap());
            listener
        }
        Err(err) => {
            tracing::error!("Failed to bind to address: {:?}", err);
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("Server error: {}", e);
    }
}

use axum::{
    extract::{FromRef, MatchedPath},
    http::{Request, StatusCode},
    routing::post,
    Router,
};
use dotenv::dotenv;
use proof_generator::controller::mev_blocker::call_mev_blocker_api;
use reqwest::Client;
use starknet::{
    core::types::FieldElement,
    signers::{LocalWallet, SigningKey},
};
use starknet_handler::{
    fact_registry::fact_registry::FactRegistry, l1_headers_store::l1_headers_store::L1HeadersStore,
};
use std::str::FromStr;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;
use tracing::info_span;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub mod handlers;
mod state;

use crate::state::AppState;
use handlers::get_storage_value::get_storage_value;

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

    let app_state = AppState {
        client: Client::new(),
        // fact_registry: fact_registry_contract, //
        // l1_headers_store: l1_headers_store_contract,
    };

    let app = Router::new()
        .route("/get-storage", post(get_storage_value))
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

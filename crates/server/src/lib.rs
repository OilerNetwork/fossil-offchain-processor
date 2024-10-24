pub mod handlers;
pub mod middlewares;
pub mod pricing_data;
pub mod types;

// src/lib.rs
use crate::middlewares::auth::simple_apikey_auth;
use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use db_access::DbConnection;
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DbConnection>,
}

pub async fn create_app(pool: PgPool) -> Router {
    let db = DbConnection { pool };
    let app_state = AppState { db: Arc::new(db) };

    let secured_routes = Router::new()
        .route(
            "/pricing_data",
            post(handlers::get_pricing_data::get_pricing_data),
        )
        .layer(from_fn_with_state(app_state.clone(), simple_apikey_auth));

    let public_routes = Router::new()
        .route("/health", get(handlers::health_check::health_check))
        .route("/api_key", post(handlers::api_key::create_api_key))
        .route(
            "/job_status/:job_id",
            get(handlers::job_status::get_job_status),
        );

    Router::new()
        .merge(secured_routes)
        .merge(public_routes)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(app_state)
}

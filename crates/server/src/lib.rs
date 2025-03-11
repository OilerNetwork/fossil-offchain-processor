#![deny(unused_crate_dependencies)]
use tracing_subscriber as _;

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
use db_access::{
    models::JobStatus,
    queries::{get_stale_in_progress_jobs, update_job_status},
    DbConnection,
};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tower_http::{
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DbConnection>,
}

pub async fn create_app(pool: PgPool) -> Router {
    let db = DbConnection { pool };
    let app_state = AppState { db: Arc::new(db) };

    // Define the CORS layer
    let allowed_origins = std::env::var("ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect::<Vec<_>>();

    let cors_layer = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_methods(AllowMethods::any()) // Allow all methods (customize as needed)
        .allow_headers(AllowHeaders::any()) // Allow all headers
        .max_age(Duration::from_secs(3600)); // Cache preflight response for 1 hour

    let secured_routes = Router::new()
        .route(
            "/pricing_data",
            post(handlers::get_pricing_data::get_pricing_data),
        )
        .layer(from_fn_with_state(app_state.clone(), simple_apikey_auth));
    //.layer(cors_layer.clone());

    let public_routes = Router::new()
        .route("/health", get(handlers::health_check::health_check))
        .route("/api_key", post(handlers::api_key::create_api_key))
        .route(
            "/job_status/{job_id}",
            get(handlers::job_status::get_job_status),
        )
        .route(
            "/latest_block",
            get(handlers::latest_block::get_latest_block_number),
        )
        .layer(CorsLayer::permissive());
    //.layer(cors_layer.clone());

    Router::new()
        .merge(secured_routes)
        .merge(public_routes)
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer) // Apply the custom CORS layer
        .with_state(app_state)
}

// Add this function to handle stale in_progress jobs
pub async fn handle_stale_jobs(db_pool: &sqlx::PgPool) {
    tracing::info!("Checking for stale in-progress jobs on startup");

    // Consider jobs in_progress for more than 10 minutes as stale
    let stale_timeout = Duration::from_secs(10 * 60);

    match get_stale_in_progress_jobs(db_pool, stale_timeout.as_secs() as i64).await {
        Ok(jobs) => {
            let count = jobs.len();
            tracing::info!("Found {} stale in-progress jobs", count);

            for job in jobs {
                tracing::info!("Marking stale job {} as failed", job.job_id);
                if let Err(e) = update_job_status(
                    db_pool,
                    &job.job_id,
                    JobStatus::Failed,
                    Some(serde_json::json!({
                        "error": "Job was interrupted by service restart"
                    })),
                )
                .await
                {
                    tracing::error!("Failed to update stale job status: {:?}", e);
                }
            }

            tracing::info!("Finished processing stale jobs");
        }
        Err(e) => {
            tracing::error!("Failed to retrieve stale in-progress jobs: {:?}", e);
        }
    }
}

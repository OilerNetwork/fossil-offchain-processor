use anyhow::Result;
use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use db_access::DbConnection;
use server::{handlers, middlewares::auth::simple_apikey_auth};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::Level;
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup database connection from another crate
    let db = DbConnection::new().await?;

    // Setup tracking aka logging.
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter::Targets::new().with_default(Level::DEBUG))
        .init();

    let app = Router::new()
        .route(
            "/job_status/:job_id",
            get(handlers::job_status::get_job_status)
                .layer(from_fn_with_state(db.clone(), simple_apikey_auth)),
        )
        .route(
            "/pricing_data",
            post(handlers::get_pricing_data::get_pricing_data)
                .layer(from_fn_with_state(db.clone(), simple_apikey_auth)),
        )
        .layer(TraceLayer::new_for_http())
        // We don't want to trace health check endpoint,
        // Since it'll become spam in our logs.
        .route("/health", get(handlers::health_check::health_check))
        .layer(CorsLayer::permissive())
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::debug!("Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

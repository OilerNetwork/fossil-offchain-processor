use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use db_access::DbConnection;
use eyre::Result;
use server::{handlers, middlewares::auth::simple_apikey_auth};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::Level;
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    let db = DbConnection::new().await?;

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter::Targets::new().with_default(Level::DEBUG))
        .init();

    // Define routes with specific middleware.
    let secured_routes = Router::new()
        .route(
            "/job_status/:job_id",
            get(handlers::job_status::get_job_status)
                .layer(from_fn_with_state(db.clone(), simple_apikey_auth)),
        )
        .route(
            "/pricing_data",
            post(handlers::get_pricing_data::get_pricing_data)
                .layer(from_fn_with_state(db.clone(), simple_apikey_auth)),
        );

    let public_routes = Router::new().route("/health", get(handlers::health_check::health_check));

    // Build the complete application router with middleware layers.
    let app = Router::new()
        .merge(secured_routes)
        .merge(public_routes)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::debug!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

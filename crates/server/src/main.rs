use dotenv::dotenv;
use server::create_app;
use sqlx::PgPool;
use std::error::Error;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    // Set up the database connection pool
    let pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    let app = create_app(pool).await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    // Configure JSON-based logging
    let fmt_layer = fmt::layer()
        .event_format(fmt::format()) // Set JSON formatting
        .with_timer(fmt::time::UtcTime::rfc_3339()) // Use UTC timestamps
        .with_thread_names(true) // Include thread names in logs
        .with_thread_ids(true); // Include thread IDs

    // Use EnvFilter for dynamic log level configuration
    // Use EnvFilter for dynamic log level configuration, including SQLx-specific filtering
    let filter_layer =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=error"));

    // Compose layers and initialize the logger
    Registry::default()
        .with(fmt_layer)
        .with(filter_layer)
        .init();

    info!("Server is listening on {}", listener.local_addr()?);
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

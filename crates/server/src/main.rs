use dotenv::dotenv;
use sqlx::PgPool;
use std::error::Error;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    let fmt_layer = fmt::layer()
        .event_format(fmt::format())
        .with_timer(fmt::time::UtcTime::rfc_3339())
        .with_thread_names(true)
        .with_thread_ids(true);

    let filter_layer = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tracing=info,sqlx=error"));

    Registry::default()
        .with(fmt_layer)
        .with(filter_layer)
        .init();

    info!("Server is listening on {}", listener.local_addr()?);

    // Handle stale in-progress jobs on startup
    server::handle_stale_jobs(&pool).await;

    // Create app after handling stale jobs
    let app = server::create_app(pool).await;

    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

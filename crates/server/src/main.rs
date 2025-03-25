use db_access::{IndexerDbConnection, OffchainProcessorDbConnection};
use dotenv::dotenv;
use server::create_app;
use std::{error::Error, sync::Arc};
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let offchain_processor_db = Arc::new(OffchainProcessorDbConnection::from_env().await?);
    let indexer_db = Arc::new(IndexerDbConnection::from_env().await?);

    let app = create_app(offchain_processor_db, indexer_db).await;
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
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

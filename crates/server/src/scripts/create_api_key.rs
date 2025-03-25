use db_access::{auth::add_api_key, OffchainProcessorDbConnection};
use eyre::Result;
use std::{env, sync::Arc};
use tracing::info;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up the tracing subscriber with INFO level
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let db = OffchainProcessorDbConnection::from_env().await?;

    let name = match env::args().nth(1) {
        Some(name) => name,
        None => {
            return Err(eyre::eyre!("Missing required argument: name"));
        }
    };

    // Generate a new API key (using UUID v4 for this example)
    let api_key = Uuid::new_v4().to_string();

    add_api_key(Arc::new(db), api_key.clone(), name).await?;

    info!("API Key Created: {}", api_key);

    Ok(())
}

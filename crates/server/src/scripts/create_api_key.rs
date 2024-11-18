use db_access::{auth::add_api_key, DbConnection};
use eyre::Result;
use std::env;
use tracing::info;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up the tracing subscriber with INFO level
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let db = DbConnection::new().await?;

    let name = env::args()
        .nth(1)
        .expect("Give a name or tag to your api key");

    // Generate a new API key (using UUID v4 for this example)
    let api_key = Uuid::new_v4().to_string();

    add_api_key(&db.pool, api_key.clone(), name).await?;

    info!("API Key Created: {}", api_key);

    Ok(())
}

use dotenv::dotenv;
use server::create_app;
use sqlx::PgPool;
use std::env;
use std::error::Error;
use tracing::{error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    // Validate required environment variables
    validate_env_vars(&[
        "SECRET_DATABASE_URL",
        "SECRET_STARKNET_RPC_URL",
        "SECRET_STARKNET_ACCOUNT_ADDRESS",
        "SECRET_STARKNET_PRIVATE_KEY",
        "SECRET_ETH_RPC_URL",
    ])?;

    let pool = PgPool::connect(&env::var("DATABASE_URL")?).await?;
    let app = create_app(pool).await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    let fmt_layer = fmt::layer()
        .event_format(fmt::format())
        .with_timer(fmt::time::UtcTime::rfc_3339())
        .with_thread_names(true)
        .with_thread_ids(true);

    let filter_layer =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=error"));

    Registry::default()
        .with(fmt_layer)
        .with(filter_layer)
        .init();

    info!("Server is listening on {}", listener.local_addr()?);
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

/// Helper function to validate required environment variables
fn validate_env_vars(required_vars: &[&str]) -> Result<(), Box<dyn Error>> {
    let mut missing_vars = Vec::new();

    for &var in required_vars {
        if env::var(var).is_err() {
            missing_vars.push(var);
        }
    }

    if !missing_vars.is_empty() {
        for var in &missing_vars {
            error!("Missing required environment variable: {}", var);
        }
        return Err(format!("Missing required environment variables: {:?}", missing_vars).into());
    }

    info!("All required environment variables are loaded.");
    Ok(())
}

use dotenv::dotenv;
use eyre::Result;
use server::create_app;
use sqlx::PgPool;
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap()).await?;
    let app = create_app(pool).await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter::Targets::new().with_default(tracing::Level::DEBUG))
        .init();

    tracing::debug!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

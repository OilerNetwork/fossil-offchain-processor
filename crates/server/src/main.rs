use anyhow::Result;
use axum::{routing::get, Router};
use db_access::DbConnection;
use server::handlers;

#[tokio::main]
async fn main() -> Result<()> {
    let db = DbConnection::new().await?;

    let app = Router::new()
        .route("/", get(handlers::root))
        .route("/pricing_data", get(handlers::get_pricing_data))
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

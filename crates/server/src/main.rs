use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use db_access::DbConnection;
use server::handlers;

#[tokio::main]
async fn main() -> Result<()> {
    let db = DbConnection::new().await?;

    let app = Router::new()
        .route("/", get(handlers::root::root))
        .route(
            "/pricing_data",
            post(handlers::get_pricing_data::get_pricing_data),
        )
        .route(
            "/callback_test",
            post(handlers::pricing_callback::pricing_callback),
        )
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

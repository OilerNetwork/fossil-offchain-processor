use axum::{routing::{get, post}, Router};
use server::handlers;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(handlers::root))
        .route("/pricing_data", get(handlers::get_pricing_data))
        .route("/reserve_price", post(handlers::get_reserve_price));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

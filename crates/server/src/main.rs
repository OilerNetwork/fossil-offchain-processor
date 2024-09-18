use axum::{routing::get, Router};
use server::handlers;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(handlers::root))
        .route("/twap", get(handlers::get_twap))
        .route("/volatility", get(handlers::get_volatility))
        .route("/reserve_price", get(handlers::get_reserve_price));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

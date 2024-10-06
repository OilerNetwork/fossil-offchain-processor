pub async fn root() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, routing::get, Router};
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_root_route_responds_ok() {
        let app = Router::new().route("/", get(root));
        let server = TestServer::new(app).expect("Failed to start the test server");

        let response = server.get("/").await;

        assert_eq!(response.status_code(), StatusCode::OK);
        assert_eq!(response.text().await, "OK");
    }
}

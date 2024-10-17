pub async fn health_check() -> &'static str {
    // TODO: should actually also check db connection and more
    // right now its essentially a liveness check.
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, routing::get, Router};

    use axum_test::TestServer;

    #[tokio::test]
    async fn test_root() {
        let app = Router::new().route("/", get(health_check));
        let server = TestServer::new(app).unwrap();

        let response = server.get("/").await;

        assert_eq!(response.status_code(), StatusCode::OK);
        assert_eq!(response.text(), "OK");
    }
}

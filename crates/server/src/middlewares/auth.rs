use crate::types::AuthErrorResponse;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use db_access::{auth::find_api_key, DbConnection};
/// A simple api key auth that checks if the key provided is in the db
/// TODO: change this to use the more comprehensive tower_http auth middleware.
pub async fn simple_apikey_auth(
    State(db): State<DbConnection>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let response_data = AuthErrorResponse {
        error: "Unauthenticated".to_string(),
    };

    if let Some(incoming_api_key) = headers.get("x-api-key") {
        if let Ok(incoming_api_key) = incoming_api_key.to_str() {
            let matching_api_key = find_api_key(&db.pool, incoming_api_key.to_string()).await;

            return match matching_api_key {
                Ok(_) => next.run(request).await,
                Err(err) => {
                    tracing::debug!("{:?}", err);
                    (StatusCode::UNAUTHORIZED, Json(response_data)).into_response()
                }
            };
        }
    }

    (StatusCode::UNAUTHORIZED, Json(response_data)).into_response()
}

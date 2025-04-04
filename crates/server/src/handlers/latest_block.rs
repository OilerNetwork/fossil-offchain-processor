use crate::types::{ErrorResponse, GetLatestBlockResponseEnum, LatestBlockResponse};
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use db_access::queries::latest_block_number;

#[axum::debug_handler]
pub async fn get_latest_block_number(
    State(state): State<AppState>,
) -> (StatusCode, Json<GetLatestBlockResponseEnum>) {
    tracing::info!("Getting the latest block number");

    match latest_block_number(state.indexer_db).await {
        Ok(Some(block_header)) => {
            tracing::info!("Latest block found: {:?}", block_header);
            if let Some(timestamp) = block_header.timestamp {
                tracing::info!("Block timestamp found: {}", timestamp);
                (
                    StatusCode::OK,
                    Json(GetLatestBlockResponseEnum::Success(LatestBlockResponse {
                        latest_block_number: block_header.number,
                        block_timestamp: if let Ok(ts) = timestamp.parse::<i64>() {
                            ts.to_string()
                        } else {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(GetLatestBlockResponseEnum::Error(ErrorResponse {
                                    error: format!(
                                        "Failed to parse block timestamp: {}",
                                        timestamp
                                    ),
                                })),
                            );
                        },
                    })),
                )
            } else {
                tracing::error!("Block timestamp is missing");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(GetLatestBlockResponseEnum::Error(ErrorResponse {
                        error: "Block timestamp is missing".to_string(),
                    })),
                )
            }
        }
        Ok(None) => {
            tracing::info!("No block found");
            (
                StatusCode::NOT_FOUND,
                Json(GetLatestBlockResponseEnum::Error(ErrorResponse {
                    error: "No block found".to_string(),
                })),
            )
        }
        Err(e) => {
            tracing::error!("Failed to fetch block: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GetLatestBlockResponseEnum::Error(ErrorResponse {
                    error: "An internal error occurred. Please try again later.".to_string(),
                })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use core::panic;

    use crate::{handlers::fixtures::TestContext, types::GetLatestBlockResponseEnum};
    use axum::{http::StatusCode, Json};

    #[tokio::test]
    async fn test_get_latest_block_not_found() {
        let ctx = TestContext::new().await;

        let (status, Json(response)) = ctx.get_latest_block().await;

        let response = match response {
            GetLatestBlockResponseEnum::Error(err_response) => err_response,
            GetLatestBlockResponseEnum::Success(_) => panic!("Unexpected response status"),
        };

        println!("Status: {}", status);
        println!("Response: {:?}", response);
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(response.error, "No block found");
    }

    #[tokio::test]
    async fn test_get_latest_block_success() {
        let ctx = TestContext::new().await;

        // Create three blocks with different timestamps
        ctx.create_block(12345, "1234567890".to_string(), 0).await;
        ctx.create_block(12346, "1234567891".to_string(), 0).await;
        let latest_block = 12347;
        let latest_timestamp = "1234567892".to_string();
        ctx.create_block(latest_block, latest_timestamp.clone(), 0)
            .await;

        let (status, Json(response)) = ctx.get_latest_block().await;

        let response = match response {
            GetLatestBlockResponseEnum::Success(success_res) => success_res,
            GetLatestBlockResponseEnum::Error(_) => panic!("Unexpected response status"),
        };

        println!("Status: {}", status);
        println!("Response: {:?}", response);
        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.latest_block_number, latest_block);
        assert_eq!(
            response.block_timestamp,
            latest_timestamp.parse::<i64>().unwrap().to_string()
        );
    }

    #[tokio::test]
    async fn test_get_latest_block_internal_error() {
        let ctx = TestContext::new().await;

        // Drop the blockheaders table to cause a database error
        sqlx::query("DROP TABLE blockheaders")
            .execute(&ctx.indexer_db.db_connection().pool)
            .await
            .expect("Failed to drop table");

        let (status, Json(response)) = ctx.get_latest_block().await;

        let response = match response {
            GetLatestBlockResponseEnum::Error(err_response) => err_response,
            GetLatestBlockResponseEnum::Success(_) => panic!("Unexpected response status"),
        };

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response.error,
            "An internal error occurred. Please try again later."
        );
    }
}

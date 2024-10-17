use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use db_access::{
    models::JobStatus,
    queries::{create_job_request, get_job_request, update_job_status},
    DbConnection,
};
use mockall::predicate::*;
use server::handlers::get_pricing_data::{get_pricing_data, PitchLakeJobRequest, PitchLakeJobRequestParams};

// Mock the database connection
mock! {
    pub DbPool {}
    impl DbConnection for DbPool {
        fn pool(&self) -> &sqlx::PgPool;
    }
}

// Helper function to create a mock DB connection
fn mock_db_connection() -> DbConnection {
    let mut mock_pool = MockDbPool::new();
    mock_pool.expect_pool().returning(|| unimplemented!());
    DbConnection::new(mock_pool)
}

// Helper function to create a sample job request
fn create_sample_job_request() -> PitchLakeJobRequest {
    PitchLakeJobRequest {
        identifiers: vec!["test_id".to_string()],
        params: PitchLakeJobRequestParams {
            twap: (0, 100),
            volatility: (0, 100),
            reserve_price: (0, 100),
        },
    }
}

#[tokio::test]
async fn test_new_job_request() {
    let db = mock_db_connection();
    
    create_job_request.mock_safe(|_, _, _| {
        Box::pin(async { Ok(()) })
    });

    let job_request = create_sample_job_request();
    let (status, response) = get_pricing_data(State(db), Json(job_request)).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(response.0.message.contains("New job request registered"));
}

#[tokio::test]
async fn test_pending_job_request() {
    let db = mock_db_connection();
    
    get_job_request.mock_safe(|_, _| {
        Box::pin(async {
            Ok(Some(db_access::models::JobRequest {
                id: "test_id".to_string(),
                status: JobStatus::Pending,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }))
        })
    });

    let job_request = create_sample_job_request();
    let (status, response) = get_pricing_data(State(db), Json(job_request)).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert!(response.0.message.contains("Job is already pending"));
}

#[tokio::test]
async fn test_completed_job_request() {
    let db = mock_db_connection();
    
    get_job_request.mock_safe(|_, _| {
        Box::pin(async {
            Ok(Some(db_access::models::JobRequest {
                id: "test_id".to_string(),
                status: JobStatus::Completed,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }))
        })
    });

    let job_request = create_sample_job_request();
    let (status, response) = get_pricing_data(State(db), Json(job_request)).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert!(response.0.message.contains("Job has already been completed"));
}

#[tokio::test]
async fn test_failed_job_request() {
    let db = mock_db_connection();
    
    get_job_request.mock_safe(|_, _| {
        Box::pin(async {
            Ok(Some(db_access::models::JobRequest {
                id: "test_id".to_string(),
                status: JobStatus::Failed,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }))
        })
    });

    update_job_status.mock_safe(|_, _, _| {
        Box::pin(async { Ok(()) })
    });

    let job_request = create_sample_job_request();
    let (status, response) = get_pricing_data(State(db), Json(job_request)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(response.0.message.contains("Previous job request failed. Reprocessing initiated"));
}

#[tokio::test]
async fn test_error_creating_job() {
    let db = mock_db_connection();
    
    create_job_request.mock_safe(|_, _, _| {
        Box::pin(async { Err(sqlx::Error::RowNotFound) })
    });

    let job_request = create_sample_job_request();
    let (status, response) = get_pricing_data(State(db), Json(job_request)).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.0.message.contains("An error occurred while creating the job"));
}

#[tokio::test]
async fn test_error_updating_failed_job() {
    let db = mock_db_connection();
    
    get_job_request.mock_safe(|_, _| {
        Box::pin(async {
            Ok(Some(db_access::models::JobRequest {
                id: "test_id".to_string(),
                status: JobStatus::Failed,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }))
        })
    });

    update_job_status.mock_safe(|_, _, _| {
        Box::pin(async { Err(sqlx::Error::RowNotFound) })
    });

    let job_request = create_sample_job_request();
    let (status, response) = get_pricing_data(State(db), Json(job_request)).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.0.message.contains("An error occurred while updating job status"));
}

#[tokio::test]
async fn test_error_querying_job_request() {
    let db = mock_db_connection();
    
    get_job_request.mock_safe(|_, _| {
        Box::pin(async { Err(sqlx::Error::RowNotFound) })
    });

    let job_request = create_sample_job_request();
    let (status, response) = get_pricing_data(State(db), Json(job_request)).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.0.message.contains("An error occurred while processing the request"));
}

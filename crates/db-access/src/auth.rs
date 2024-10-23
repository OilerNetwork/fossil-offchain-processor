use crate::models::ApiKey;
use sqlx::PgPool;

pub async fn add_api_key(pool: &PgPool, api_key: String, name: String) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO api_keys (key, name, created_at) VALUES ($1, $2, now())",
        api_key,
        name
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_api_key(pool: &PgPool, key: String) -> Result<ApiKey, sqlx::Error> {
    tracing::debug!("Searching for API key: {}", key);
    let api_key = sqlx::query_as!(
        ApiKey,
        r#"
        SELECT key, name as "name?"
        FROM api_keys
        WHERE key = $1
        "#,
        key
    )
    .fetch_one(pool)
    .await?;

    tracing::debug!("Found API key: {:?}", api_key);
    Ok(api_key)
}

pub async fn validate_api_key(pool: &PgPool, api_key: &str) -> Result<(), sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT key
        FROM api_keys
        WHERE key = $1
        "#,
        api_key
    )
    .fetch_optional(pool)
    .await?;

    match result {
        Some(_) => Ok(()),
        None => Err(sqlx::Error::RowNotFound),
    }
}

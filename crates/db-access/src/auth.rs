use crate::models::ApiKey;
use sqlx::{Error, PgPool};

pub async fn add_api_key(pool: &PgPool, key: String, name: String) -> Result<(), Error> {
    sqlx::query!(
        r#"INSERT INTO api_keys (key, name) VALUES ($1, $2)"#,
        key,
        name
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn find_api_key(pool: &PgPool, key: String) -> Result<ApiKey, Error> {
    let result = sqlx::query_as!(
        ApiKey,
        r#"
        SELECT
            key,
            name
        FROM api_keys
        WHERE key = $1
    "#,
        key
    )
    .fetch_one(pool)
    .await?;

    Ok(result)
}

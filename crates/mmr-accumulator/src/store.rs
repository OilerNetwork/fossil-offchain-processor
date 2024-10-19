use accumulators::store::{sqlite::SQLiteStore, StoreError};
use eyre::Result;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[allow(dead_code)]
pub struct StoreFactory;

#[allow(dead_code)]
impl StoreFactory {
    pub async fn create_store(path: &str, id: Option<&str>) -> Result<SQLiteStore, StoreError> {
        SQLiteStore::new(path, Some(true), id)
            .await
            .map_err(StoreError::SQLite)
    }
}

#[allow(dead_code)]
pub struct StoreManager {
    stores: Mutex<HashMap<String, Arc<SQLiteStore>>>,
}

impl StoreManager {
    pub async fn new(path: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(path).await?;

        // Create the mmr_metadata table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS mmr_metadata (
                mmr_id TEXT PRIMARY KEY
            )
            "#,
        )
        .execute(&pool)
        .await?;

        let manager = StoreManager {
            stores: Mutex::new(HashMap::new()),
        };

        // Initialize the value-to-index table
        manager.initialize_value_index_table(&pool).await?;

        Ok(manager)
    }

    pub async fn initialize_value_index_table(&self, pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS value_index_map (
                value TEXT PRIMARY KEY,
                element_index INTEGER NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn insert_value_index_mapping(
        &self,
        pool: &SqlitePool,
        value: &str,
        element_index: usize,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO value_index_map (value, element_index)
            VALUES (?, ?)
            "#,
        )
        .bind(value)
        .bind(element_index as i64) // SQLite uses i64 for integers
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Retrieves the element index based on the given hash value
    #[allow(dead_code)]
    pub async fn get_element_index_for_value(
        &self,
        pool: &SqlitePool,
        value: &str,
    ) -> Result<Option<usize>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT element_index FROM value_index_map WHERE value = ?
            "#,
        )
        .bind(value)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            let element_index: i64 = row.get("element_index");
            Ok(Some(element_index as usize))
        } else {
            Ok(None)
        }
    }

    /// Retrieves the stored value for the given element index, abstracting away the MMR ID
    #[allow(dead_code)]
    pub async fn get_value_for_element_index(
        &self,
        pool: &SqlitePool,
        element_index: usize,
    ) -> Result<Option<String>, sqlx::Error> {
        let element_index_str = element_index.to_string();

        // Query the store for the value associated with the given element_index
        let row = sqlx::query(
            r#"
            SELECT value FROM store WHERE key LIKE ?
            "#,
        )
        .bind(format!("%:hashes:{}", element_index_str)) // Match the key pattern using LIKE
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            let stored_value: String = row.get("value");
            Ok(Some(stored_value))
        } else {
            Ok(None)
        }
    }
}

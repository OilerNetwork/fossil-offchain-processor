#![deny(unused_crate_dependencies)]
use tokio as _;

pub mod auth;
pub mod models;
pub mod queries;
pub mod rpc;
pub mod utils;

use dotenv::dotenv;
use eyre::{eyre, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::env;
use std::sync::Arc;

#[derive(Debug)]
pub struct DbConnection {
    pub pool: Pool<Postgres>,
}

// Use Arc to allow thread-safe cloning
impl DbConnection {
    pub async fn new() -> Result<Arc<Self>> {
        dotenv().ok();
        let database_url =
            env::var("DATABASE_URL").map_err(|_| eyre!("DATABASE_URL must be set"))?;

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?;

        Ok(Arc::new(Self { pool }))
    }
}

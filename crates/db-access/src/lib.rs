#![deny(unused_crate_dependencies)]
use tokio as _;

pub mod auth;
pub mod models;
pub mod queries;
pub mod rpc;
pub mod utils;

use dotenv::dotenv;
use sqlx::postgres::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::env;

#[derive(Clone, Debug)]
pub struct DbConnection {
    pub pool: PgPool,
}

impl DbConnection {
    pub async fn new() -> Result<Self, sqlx::Error> {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await?;

        Ok(Self { pool })
    }
}

mod models;
pub mod queries;

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

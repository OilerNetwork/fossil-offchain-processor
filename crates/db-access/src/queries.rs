use sqlx::{types::BigDecimal, Error, PgPool};

use crate::models::{BlockHeader, BlockHeaderSubset, JobRequest, JobStatus, Transaction};

pub async fn get_transactions_by_block_number(
    pool: &PgPool,
    block_number: i64,
) -> Result<Vec<Transaction>, Error> {
    let transactions = sqlx::query_as!(
        Transaction,
        r#"
        SELECT block_number, transaction_hash, transaction_index, from_addr, to_addr, value, gas_price,
               max_priority_fee_per_gas, max_fee_per_gas, gas, chain_id
        FROM public.transactions
        WHERE block_number = $1
        "#,
        block_number
    )
    .fetch_all(pool)
    .await?;

    Ok(transactions)
}

pub async fn get_base_fees_between_blocks(
    pool: &PgPool,
    start_block: i64,
    end_block: i64,
) -> Result<Vec<BlockHeaderSubset>, Error> {
    let headers = sqlx::query_as!(
        BlockHeaderSubset,
        r#"
        SELECT number, base_fee_per_gas, timestamp
        FROM blockheaders
        WHERE number BETWEEN $1 AND $2
        ORDER BY number ASC
        "#,
        start_block,
        end_block
    )
    .fetch_all(pool)
    .await?;

    Ok(headers)
}

pub async fn get_avg_base_fee(
    pool: &PgPool,
    start_block: i64,
    end_block: i64,
) -> Result<Option<BigDecimal>, Error> {
    let avg_base_fee = sqlx::query_scalar!(
        r#"
        SELECT AVG(CAST(base_fee_per_gas AS NUMERIC))
        FROM blockheaders
        WHERE number BETWEEN $1 AND $2
        "#,
        start_block,
        end_block
    )
    .fetch_one(pool)
    .await?;

    Ok(avg_base_fee)
}

pub async fn get_base_fee_volatility(
    pool: &PgPool,
    start_block: i64,
    end_block: i64,
) -> Result<Option<BigDecimal>, Error> {
    let volatility = sqlx::query_scalar!(
        r#"
        SELECT STDDEV(CAST(base_fee_per_gas AS NUMERIC))
        FROM blockheaders
        WHERE number BETWEEN $1 AND $2
        "#,
        start_block,
        end_block
    )
    .fetch_one(pool)
    .await?;

    Ok(volatility)
}

pub async fn get_reserve_price(pool: &PgPool, x: i64, y: i64) -> Result<Option<BigDecimal>, Error> {
    let reserve_price = sqlx::query_scalar!(
        r#"
        WITH twap AS (
            SELECT AVG(CAST(base_fee_per_gas AS NUMERIC)) AS avg_base_fee
            FROM blockheaders
            WHERE number BETWEEN 12345 AND 14345
        ),
        volatility AS (
            SELECT STDDEV(CAST(base_fee_per_gas AS NUMERIC)) AS base_fee_volatility
            FROM blockheaders
            WHERE number BETWEEN $1 AND $2
        )
        SELECT (avg_base_fee + base_fee_volatility) AS reserve_price
        FROM twap, volatility
        "#,
        x,
        y
    )
    .fetch_one(pool)
    .await?;

    Ok(reserve_price)
}

pub async fn get_twap_and_volatility(
    pool: &PgPool,
    x: i64,
    y: i64,
) -> Result<(Option<BigDecimal>, Option<BigDecimal>), Error> {
    let row = sqlx::query!(
        r#"
        SELECT AVG(CAST(base_fee_per_gas AS NUMERIC)) AS twap,
               STDDEV(CAST(base_fee_per_gas AS NUMERIC)) AS volatility
        FROM blockheaders
        WHERE number BETWEEN $1 AND $2
        "#,
        x,
        y
    )
    .fetch_one(pool)
    .await?;

    Ok((row.twap, row.volatility))
}

pub async fn get_block_by_number(
    pool: &PgPool,
    block_number: i64,
) -> Result<Option<BlockHeader>, Error> {
    let block = sqlx::query_as!(
        BlockHeader,
        r#"
        SELECT 
            block_hash, 
            number, 
            gas_limit, 
            gas_used, 
            base_fee_per_gas, 
            nonce, 
            transaction_root, 
            receipts_root, 
            state_root,
            timestamp
        FROM blockheaders
        WHERE number = $1
        "#,
        block_number
    )
    .fetch_optional(pool)
    .await?;

    Ok(block)
}

pub async fn get_block_headers_by_time_range(
    pool: &PgPool,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Result<Vec<BlockHeader>, Error> {
    let headers = sqlx::query_as!(
        BlockHeader,
        r#"
        SELECT 
            block_hash, 
            number, 
            gas_limit, 
            gas_used, 
            base_fee_per_gas, 
            nonce, 
            transaction_root, 
            receipts_root, 
            state_root,
            timestamp
        FROM blockheaders
        WHERE timestamp BETWEEN $1 AND $2
        ORDER BY number ASC
        "#,
        start_timestamp,
        end_timestamp
    )
    .fetch_all(pool)
    .await?;

    Ok(headers)
}

pub async fn create_job_request(
    pool: &PgPool,
    job_id: &str,
    status: JobStatus,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO job_requests (job_id, status) VALUES ($1, $2)
        "#,
        job_id,
        status.as_str()
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_job_request(
    pool: &PgPool,
    job_id: &str,
) -> Result<Option<JobRequest>, sqlx::Error> {
    sqlx::query_as!(
        JobRequest,
        r#"
        SELECT job_id, status as "status: JobStatus"
        FROM job_requests
        WHERE job_id = $1
        "#,
        job_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn update_job_status(
    pool: &PgPool,
    job_id: &str,
    status: JobStatus,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE job_requests
        SET status = $1
        WHERE job_id = $2
        "#,
        status.as_str(),
        job_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

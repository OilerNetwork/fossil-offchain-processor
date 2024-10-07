#[derive(sqlx::FromRow, Debug)]
pub struct BlockHeader {
    pub block_hash: Option<String>,
    pub number: i64,
    pub gas_limit: Option<i64>,
    pub gas_used: Option<i64>,
    pub base_fee_per_gas: Option<String>,
    pub nonce: Option<String>,
    pub transaction_root: Option<String>,
    pub receipts_root: Option<String>,
    pub state_root: Option<String>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Transaction {
    pub block_number: Option<i64>,
    pub transaction_hash: String,
    pub transaction_index: Option<i32>,
    pub from_addr: Option<String>,
    pub to_addr: Option<String>,
    pub value: Option<String>,
    pub gas_price: Option<String>,
    pub max_priority_fee_per_gas: Option<String>,
    pub max_fee_per_gas: Option<String>,
    pub gas: Option<String>,
    pub chain_id: Option<String>,
}

#[derive(Debug)]
pub struct BlockHeaderSubset {
    pub number: i64,
    pub base_fee_per_gas: Option<String>,
    pub timestamp: Option<i64>,
}

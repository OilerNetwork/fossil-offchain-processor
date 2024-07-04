//! Module for handling Ethereum transactions.
//!
//! This module provides functionalities to sign, send, and simulate Ethereum transactions,
//! as well as to wait for transaction receipts.

use super::error::TransactionError;
use ethers::providers::{JsonRpcClient, Middleware, PendingTransaction};
use ethers::signers::{LocalWallet, WalletError};
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{
    BlockId, BlockNumber, Bytes, Eip1559TransactionRequest, TransactionReceipt, H160,
};
use std::sync::Arc;
use tracing::instrument;

/// Signs and sends a transaction, and bumps gas if necessary.
///
/// # Arguments
///
/// * `tx` - The typed transaction to be signed and sent.
/// * `wallet_key` - The wallet key used for signing the transaction.
/// * `middleware` - The middleware to interact with the Ethereum network.
///
/// # Returns
///
/// A result containing the transaction receipt on success, or a `TransactionError` on failure.
///
/// # Errors
///
/// This function returns an error if the transaction signing or sending fails,
/// or if there are insufficient funds in the wallet.
///
/// # Example
///
/// ```
/// let receipt = sign_and_send_transaction(tx, &wallet_key, middleware).await?;
/// ```
#[instrument(skip(wallet_key, middleware))]
pub async fn sign_and_send_transaction<M: Middleware>(
    tx: TypedTransaction,
    wallet_key: &LocalWallet,
    middleware: Arc<M>,
) -> Result<TransactionReceipt, TransactionError<M>> {
    tracing::info!("Signing tx");
    let signed_tx = raw_signed_transaction(tx.clone(), wallet_key)?;
    tracing::info!("Sending tx");
    match middleware.send_raw_transaction(signed_tx.clone()).await {
        Ok(pending_tx) => {
            let tx_hash = pending_tx.tx_hash();
            tracing::info!(?tx_hash, "Pending tx");

            return wait_for_tx_receipt(pending_tx).await;
        }
        Err(err) => {
            let error_string = err.to_string();
            if error_string.contains("insufficient funds") {
                tracing::error!("Insufficient funds");
                return Err(TransactionError::InsufficientWalletFunds);
            }
            return Err(TransactionError::MiddlewareError(err));
        }
    }
}

/// Fills and simulates an EIP-1559 transaction.
///
/// # Arguments
///
/// * `calldata` - The calldata for the transaction.
/// * `to` - The recipient address of the transaction.
/// * `from` - The sender address of the transaction.
/// * `chain_id` - The chain ID of the Ethereum network.
/// * `middleware` - The middleware to interact with the Ethereum network.
/// * `value` - The value to be sent in the transaction.
///
/// # Returns
///
/// A result containing the filled and simulated typed transaction on success, or a `TransactionError` on failure.
///
/// # Errors
///
/// This function returns an error if gas estimation, transaction filling, or simulation fails.
///
/// # Example
///
/// ```
/// let tx = fill_and_simulate_eip1559_transaction(calldata, to, from, chain_id, middleware, value).await?;
/// ```
#[instrument(skip(middleware))]
pub async fn fill_and_simulate_eip1559_transaction<M: Middleware>(
    calldata: Bytes,
    to: H160,
    from: H160,
    chain_id: u64,
    middleware: Arc<M>,
    value: u32,
) -> Result<TypedTransaction, TransactionError<M>> {
    let (max_fee_per_gas, max_priority_fee_per_gas) = middleware
        .estimate_eip1559_fees(None)
        .await
        .map_err(TransactionError::MiddlewareError)?;

    tracing::info!(
        ?max_fee_per_gas,
        ?max_priority_fee_per_gas,
        "Estimated gas fees"
    );

    let nonce = middleware
        .get_transaction_count(from, Some(BlockId::Number(BlockNumber::Latest)))
        .await
        .unwrap();

    let mut tx: TypedTransaction = Eip1559TransactionRequest::new()
        .data(calldata.clone())
        .to(to)
        .from(from)
        .chain_id(chain_id)
        .max_priority_fee_per_gas(max_priority_fee_per_gas)
        .max_fee_per_gas(max_fee_per_gas)
        .value(value)
        .nonce(nonce)
        .into();

    middleware
        .fill_transaction(&mut tx, None)
        .await
        .map_err(TransactionError::MiddlewareError)?;

    tx.set_gas(tx.gas().unwrap() * 150 / 100);

    let tx_gas = tx.gas().expect("Could not get tx gas");
    tracing::info!(?tx_gas, "Gas limit set");

    middleware
        .call(&tx, None)
        .await
        .map_err(TransactionError::MiddlewareError)?;

    tracing::info!("Successfully simulated tx");

    Ok(tx)
}

/// Waits for a transaction receipt.
///
/// # Arguments
///
/// * `pending_tx` - The pending transaction for which to wait for the receipt.
///
/// # Returns
///
/// A result containing the transaction receipt on success, or a `TransactionError` on failure.
///
/// # Errors
///
/// This function returns an error if the transaction receipt is not found or the provider fails.
///
/// # Example
///
/// ```
/// let receipt = wait_for_tx_receipt(pending_tx).await?;
/// ```
#[instrument]
pub async fn wait_for_tx_receipt<'a, M: Middleware, P: JsonRpcClient>(
    pending_tx: PendingTransaction<'a, P>,
) -> Result<TransactionReceipt, TransactionError<M>> {
    let tx_hash = pending_tx.tx_hash();

    if let Some(tx_receipt) = pending_tx.await.map_err(TransactionError::ProviderError)? {
        tracing::info!(?tx_receipt, "Tx receipt received");

        return Ok(tx_receipt);
    }
    return Err(TransactionError::TxReceiptNotFound(tx_hash));
}

/// Signs a raw transaction.
///
/// # Arguments
///
/// * `tx` - The typed transaction to be signed.
/// * `wallet_key` - The wallet key used for signing the transaction.
///
/// # Returns
///
/// A result containing the signed transaction bytes on success, or a `WalletError` on failure.
///
/// # Errors
///
/// This function returns an error if the transaction signing fails.
///
/// # Example
///
/// ```
/// let signed_tx = raw_signed_transaction(tx, &wallet_key)?;
/// ```
pub fn raw_signed_transaction(
    tx: TypedTransaction,
    wallet_key: &LocalWallet,
) -> Result<Bytes, WalletError> {
    Ok(tx.rlp_signed(&wallet_key.sign_transaction_sync(&tx)?))
}

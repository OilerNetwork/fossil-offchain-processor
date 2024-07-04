//! Module for sending messages from Layer 1 to Layer 2 in an Ethereum network context.
//!
//! This module provides functionality to interact with Layer 1 contracts and send data to Layer 2 contracts.

use ethers::signers::Signer;
use ethers::{providers::Middleware, signers::LocalWallet};
use primitive_types::H160;
use std::{ops::Sub, sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::instrument;

use crate::relayer::transaction;

use super::abi;
use super::error::RelayerError;

/// Struct for sending messages from Layer 1 to Layer 2.
#[derive(Debug)]
pub struct L1MessageSender<M: Middleware + 'static> {
    /// Address for the message sender contract on Layer 1.
    l1_message_sender: H160,
    /// Wallet responsible for sending `sendLatestParentHashToL2`.
    wallet: LocalWallet,
    /// Middleware to interact with Layer 1.
    l1_middleware: Arc<M>,
    /// Time delay between `sendLatestParentHashToL2()` transactions.
    pub relaying_period: Duration,
}

impl<M> L1MessageSender<M>
where
    M: Middleware + 'static,
    RelayerError<M>: From<<M as Middleware>::Error>,
{
    /// Creates a new `L1MessageSender`.
    ///
    /// # Arguments
    ///
    /// * `l1_message_sender` - Address for the state bridge contract on Layer 1.
    /// * `wallet` - Wallet responsible for sending `propagateRoot` transactions.
    /// * `l1_middleware` - Middleware to interact with Layer 1.
    /// * `relaying_period` - Duration between successive `sendLatestParentHashToL2()` invocations.
    ///
    /// # Returns
    ///
    /// A result containing the new `L1MessageSender` on success, or a `RelayerError` on failure.
    ///
    /// # Example
    ///
    /// ```
    /// let sender = L1MessageSender::new(l1_address, wallet, middleware, Duration::from_secs(30))?;
    /// ```
    pub fn new(
        l1_message_sender: H160,
        wallet: LocalWallet,
        l1_middleware: Arc<M>,
        relaying_period: Duration,
    ) -> Result<Self, RelayerError<M>> {
        Ok(Self {
            l1_message_sender,
            wallet,
            l1_middleware,
            relaying_period,
        })
    }

    /// Spawns a `L1MessageSender` task to call `sendLatestParentHashToL2()` at each new block.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to be sent with the transaction.
    ///
    /// # Returns
    ///
    /// A `JoinHandle` to the spawned task.
    ///
    /// # Example
    ///
    /// ```
    /// let handle = sender.spawn(100);
    /// ```
    #[instrument(skip(self))]
    pub fn spawn(&self, value: u32) -> JoinHandle<Result<(), RelayerError<M>>> {
        let l1_message_sender = self.l1_message_sender;
        let relaying_period = self.relaying_period;
        let wallet = self.wallet.clone();
        let l1_middleware = self.l1_middleware.clone();

        tracing::info!(?l1_message_sender, ?relaying_period, "Spawning bridge");

        tokio::spawn(async move {
            let mut last_propagation = Instant::now().sub(relaying_period);

            let mut last_block = l1_middleware.get_block_number().await? - 1;

            loop {
                // Sleep
                tokio::time::sleep(relaying_period).await;
                tracing::info!(?l1_message_sender, "Sleep time elapsed");

                let time_since_last_propagation = Instant::now() - last_propagation;
                let current_block = l1_middleware.get_block_number().await?;

                if time_since_last_propagation >= relaying_period && last_block != current_block {
                    tracing::info!(?l1_message_sender, "Relaying period elapsed");

                    tracing::info!(?l1_message_sender, "Sending hash to L2");

                    Self::send_latest_parent_hash_to_l2(
                        l1_message_sender,
                        l1_middleware.clone(),
                        &wallet,
                        value,
                    )
                    .await?;

                    last_propagation = Instant::now();
                    last_block = current_block;
                }
            }
        })
    }

    /// Sends the latest parent hash to Layer 2.
    ///
    /// # Arguments
    ///
    /// * `l1_state_bridge` - The address of the Layer 1 state bridge.
    /// * `l1_middleware` - Middleware to interact with Layer 1.
    /// * `wallet` - The wallet to sign the transaction.
    /// * `value` - The value to be sent with the transaction.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    ///
    /// # Errors
    ///
    /// This function returns an error if transaction simulation or sending fails.
    ///
    /// # Example
    ///
    /// ```
    /// sender.send_latest_parent_hash_to_l2(l1_state_bridge, middleware, &wallet, 100).await?;
    /// ```
    pub async fn send_latest_parent_hash_to_l2(
        l1_state_bridge: H160,
        l1_middleware: Arc<M>,
        wallet: &LocalWallet,
        value: u32,
    ) -> Result<(), RelayerError<M>> {
        let calldata = abi::L1MESSAGESENDER_ABI
            .function("sendLatestParentHashToL2")?
            .encode_input(&[])?;

        let tx = transaction::fill_and_simulate_eip1559_transaction(
            calldata.into(),
            l1_state_bridge,
            wallet.address(),
            wallet.chain_id(),
            l1_middleware.clone(),
            value,
        )
        .await?;

        transaction::sign_and_send_transaction(tx, wallet, l1_middleware).await?;

        Ok(())
    }
}

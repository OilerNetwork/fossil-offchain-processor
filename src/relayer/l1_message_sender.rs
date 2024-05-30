use std::{ops::Sub, sync::Arc, time::Duration};

use ethers::{
    providers::{Middleware, MiddlewareError},
    signers::LocalWallet,
};
use primitive_types::H160;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::instrument;

use super::abi;

pub struct L1MessageSender<M: Middleware + 'static> {
    // Address for the state bridge contract on layer 1
    l1_state_bridge: H160,
    // Wallet responsible for sending `propagateRoot` transactions
    wallet: LocalWallet,
    // Middleware to interact with layer 1
    l1_middleware: Arc<M>,
    /// Time delay between `propagateRoot()` transactions
    pub relaying_period: Duration,
    /// The number of block confirmations before a `propagateRoot()` transaction is considered finalized
    pub block_confirmations: usize,
}

impl<M: Middleware> L1MessageSender<M> {
    /// # Arguments
    ///
    /// * l1_state_bridge - Address for the state bridge contract on layer 1.
    /// * wallet - Wallet responsible for sending `propagateRoot` transactions.
    /// * l1_middleware - Middleware to interact with layer 1.
    /// * relaying_period - Duration between successive propagateRoot() invocations.
    /// * block_confirmations - Number of block confirmations required to consider a propagateRoot() transaction as finalized.
    pub fn new(
        l1_state_bridge: H160,
        wallet: LocalWallet,
        l1_middleware: Arc<M>,
        relaying_period: Duration,
        block_confirmations: usize,
    ) -> Result<Self, L1MessageSender<M>> {
        Ok(Self {
            l1_state_bridge,
            wallet,
            l1_middleware,
            relaying_period,
            block_confirmations,
        })
    }

    /// Spawns a `L1MessageSender` task to listen for `TreeChanged` events from `WorldRoot` and propagate new roots.
    #[instrument(skip(self))]
    pub fn spawn(&self, value: u32) -> JoinHandle<Result<(), <M as Middleware>::Error>>
    where
        <M as Middleware>::Error: From<ethers::abi::Error>,
    {
        let l1_state_bridge = self.l1_state_bridge;
        let relaying_period = self.relaying_period;
        let block_confirmations = self.block_confirmations;
        let wallet = self.wallet.clone();
        let l1_middleware = self.l1_middleware.clone();

        tracing::info!(
            ?l1_state_bridge,
            ?relaying_period,
            ?block_confirmations,
            "Spawning bridge"
        );

        tokio::spawn(async move {
            let mut last_propagation = Instant::now().sub(relaying_period);

            loop {
                // Sleep
                tokio::time::sleep(relaying_period).await;
                tracing::info!(?l1_state_bridge, "Sleep time elapsed");

                let time_since_last_propagation = Instant::now() - last_propagation;

                if time_since_last_propagation >= relaying_period {
                    tracing::info!(?l1_state_bridge, "Relaying period elapsed");

                    tracing::info!(?l1_state_bridge, "Propagating root");

                    Self::send_exact_parent_hash_to_l2(
                        l1_state_bridge,
                        &wallet,
                        block_confirmations,
                        l1_middleware.clone(),
                        value,
                    )
                    .await?;

                    last_propagation = Instant::now();
                }
            }
        })
    }

    pub async fn send_exact_parent_hash_to_l2(
        l1_state_bridge: H160,
        wallet: &LocalWallet,
        block_confirmations: usize,
        l1_middleware: Arc<M>,
        value: u32,
    ) -> Result<(), <M as Middleware>::Error>
    where
        <M as Middleware>::Error: From<ethers::abi::Error>,
    {
        let calldata = abi::L1MESSAGESENDER_ABI
            .function("sendExactParentHashToL2")?
            .encode_input(&[])?;
        Ok(())
    }
}

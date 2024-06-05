use std::{ops::Sub, sync::Arc, time::Duration};

use ethers::signers::Signer;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::Eip1559TransactionRequest;
use ethers::{providers::Middleware, signers::LocalWallet};
use primitive_types::H160;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::instrument;

use super::abi;
use super::error::RelayerError;

#[derive(Debug)]
pub struct L1MessageSender<M: Middleware + 'static> {
    // Address for the message sedner contract on layer 1
    l1_message_sender: H160,
    // Wallet responsible for sending `sendLatestParentHashToL2`
    wallet: LocalWallet,
    // Middleware to interact with layer 1
    l1_middleware: Arc<M>,
    /// Time delay between `sendLatestParentHashToL2()` transactions
    pub relaying_period: Duration,
}

impl<M> L1MessageSender<M>
where
    M: Middleware + 'static,
    RelayerError<M>: From<<M as Middleware>::Error>,
{
    /// # Arguments
    ///
    /// * l1_message_sender - Address for the state bridge contract on layer 1.
    /// * wallet - Wallet responsible for sending `propagateRoot` transactions.
    /// * l1_middleware - Middleware to interact with layer 1.
    /// * relaying_period - Duration between successive `sendLatestParentHashToL2() invocations.
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

    /// Spawns a `L1MessageSender` task to call `sendLatestParentHashToL2()` at each new block
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

                if time_since_last_propagation >= relaying_period
                    && last_block != l1_middleware.get_block_number().await?
                {
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
                    last_block = l1_middleware.get_block_number().await?;
                }
            }
        })
    }

    pub async fn send_latest_parent_hash_to_l2(
        l1_state_bridge: H160,
        l1_middleware: Arc<M>,
        wallet: &LocalWallet,
        value: u32,
    ) -> Result<(), RelayerError<M>> {
        let calldata = abi::L1MESSAGESENDER_ABI
            .function("sendLatestParentHashToL2")?
            .encode_input(&[])?;

        println!("Value: {:?}", value);

        let (max_fee_per_gas, max_priority_fee_per_gas) =
            l1_middleware.estimate_eip1559_fees(None).await?;

        tracing::info!(
            ?max_fee_per_gas,
            ?max_priority_fee_per_gas,
            "Estimated gas fees"
        );

        let tx: TypedTransaction = Eip1559TransactionRequest::new()
            .data(calldata)
            .to(l1_state_bridge)
            .from(wallet.address())
            .value(value)
            .into();

        l1_middleware.send_transaction(tx, None).await?;

        Ok(())
    }
}

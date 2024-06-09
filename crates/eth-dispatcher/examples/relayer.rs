use std::{sync::Arc, time::Duration};

use eth_dispatcher::relayer::l1_message_sender::L1MessageSender;
use ethers::{
    providers::{Http, Provider, ProviderExt},
    signers::{LocalWallet, Signer},
};
use primitive_types::H160;

#[tokio::main]
async fn main() {
    test_relay().await;
}

async fn test_relay() {
    dotenv::dotenv().ok();
    // Environment Variables
    let http_local = dotenv::var("LOCAL_ETH_RPC_URL").unwrap();
    let test_private_key = dotenv::var("ACCOUNT_PRIVATE_KEY").unwrap();

    println!("Local ETH RPC URL: {}", http_local);
    println!("Account Private Key: {}", test_private_key);

    // Interfacing Setup
    let test_wallet: LocalWallet = test_private_key
        .parse::<LocalWallet>()
        .unwrap()
        .with_chain_id(31337 as u64);
    let provider: Arc<Provider<Http>> = Arc::new(Provider::<Http>::connect(&http_local).await);

    let contract_address = dotenv::var("L1_MESSAGE_SENDER_ADDRESS")
        .unwrap()
        .parse::<H160>()
        .unwrap();

    let relaying_period = Duration::new(5, 0);
    const DEFAULT_GAS: u32 = 100000;

    let l1_message_sender = L1MessageSender::new(
        contract_address,
        test_wallet,
        provider.clone(),
        relaying_period,
    )
    .unwrap();

    let join_handle = l1_message_sender.spawn(DEFAULT_GAS);

    match join_handle.await {
        Ok(Ok(result)) => println!("Task result: {:?}", result),
        Ok(Err(e)) => eprintln!("Task error: {:?}", e),
        Err(e) => eprintln!("Join error: {:?}", e),
    }
}

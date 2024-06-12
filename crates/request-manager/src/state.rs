use reqwest::Client;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub client: Client,
    pub storage_cache: Arc<Mutex<HashMap<String, String>>>,
    // pub starknet_handler: StarknetContractHandler,
    // pub proof_generator: EthereumProofGenerator,
}

use reqwest::Client;

use starknet_handler::{
    fact_registry::fact_registry::FactRegistry, l1_headers_store::l1_headers_store::L1HeadersStore,
};

#[derive(Clone)]
pub struct AppState {
    pub client: Client,
    // pub fact_registry: FactRegistry,
    // pub l1_headers_store: L1HeadersStore,
}

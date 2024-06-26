use reqwest::Client;

#[derive(Clone)]
pub struct AppState {
    pub client: Client,
    // pub fact_registry: FactRegistry,
    // pub l1_headers_store: L1HeadersStore,
}

use primitive_types::H160;
use starknet::{
    accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount},
    core::{
        chain_id,
        types::{contract::SierraClass, FieldElement},
        utils::get_selector_from_name,
    },
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Url,
    },
    signers::LocalWallet,
};

pub struct FactRegistry {
    provider: JsonRpcClient<HttpTransport>,
    signer: LocalWallet,
    _contract_artifact: SierraClass,
    fact_registry: H160,
}

#[allow(dead_code)]
impl FactRegistry {
    pub fn new(rpc: &str, fact_registry: H160, signer: LocalWallet) -> Self {
        let url = Url::parse(rpc).unwrap();
        let provider = JsonRpcClient::new(HttpTransport::new(url));

        let contract_artifact = include_bytes!("../artifacts/FactRegistry.json").to_vec();
        let _contract_artifact = serde_json::from_slice::<SierraClass>(&contract_artifact).unwrap();
        Self {
            provider,
            signer,
            _contract_artifact,
            fact_registry,
        }
    }

    pub async fn call(&self) {
        let account = SingleOwnerAccount::new(
            &self.provider,
            &self.signer,
            FieldElement::from_hex_be(std::str::from_utf8(self.fact_registry.as_bytes()).unwrap())
                .unwrap(),
            chain_id::SEPOLIA,
            ExecutionEncoding::New,
        );

        let _result = account
            .execute(vec![Call {
                to: account.address(),
                selector: get_selector_from_name("mint").unwrap(),
                calldata: vec![],
            }])
            .send()
            .await;
    }
}

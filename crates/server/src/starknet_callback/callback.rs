use std::env;

use anyhow::{Ok, Result};
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    core::{chain_id, types::Call, utils::get_selector_from_name},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Url},
    signers::{LocalWallet, SigningKey},
};
use starknet_crypto::Felt;

#[derive(Debug)]
pub struct FossilStarknetAccount {
    pub account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
}

impl Default for FossilStarknetAccount {
    fn default() -> Self {
        Self::new()
    }
}

impl FossilStarknetAccount {
    pub fn new() -> Self {
        let rpc_url = env::var("RPC_URL").expect("RPC_URL should be provided as env vars.");
        let account_private_key =
            env::var("PRIVATE_KEY").expect("PRIVATE_KEY should be provided as env vars.");
        let account_address =
            env::var("ACCOUNT_ADDRESS").expect("ACCOUNT_ADDRESS should be provided as env vars.");

        let provider = JsonRpcClient::new(HttpTransport::new(
            Url::parse(&rpc_url).expect("Invalid rpc url provided"),
        ));

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            Felt::from_hex(&account_private_key).expect("Invalid private key provided"),
        ));

        Self {
            account: SingleOwnerAccount::new(
                provider,
                signer,
                Felt::from_hex(&account_address).expect("Invalid address provided"),
                chain_id::SEPOLIA,
                ExecutionEncoding::New,
            ),
        }
    }

    pub async fn callback_to_contract(&self, contract_address: Felt) -> Result<Felt> {
        let tx = self
            .account
            .execute_v3(vec![Call {
                selector: get_selector_from_name("fossil_callback").unwrap(),
                calldata: vec![],
                to: contract_address,
            }])
            .send()
            .await?;

        Ok(tx.transaction_hash)
    }
}

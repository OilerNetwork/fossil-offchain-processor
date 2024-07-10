use starknet::{
    accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount},
    core::{
        types::{BlockId, BlockTag, Felt, FunctionCall, InvokeTransactionResult},
        utils::get_selector_from_name,
    },
    macros::felt,
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, Url},
    signers::LocalWallet,
};

use crate::{
    error::{FieldElementParseError, HandlerError},
    util::get_high_and_low,
};

pub struct L1HeadersStore {
    provider: JsonRpcClient<HttpTransport>,
    signer: LocalWallet,
    l1_headers_store: Felt,
    owner_account: Felt,
}

impl L1HeadersStore {
    pub fn new(
        rpc: &str,
        l1_headers_store: Felt,
        signer: LocalWallet,
        owner_account: Felt,
    ) -> Self {
        let url = Url::parse(rpc).unwrap();
        let provider = JsonRpcClient::new(HttpTransport::new(url));

        Self {
            provider,
            signer,
            l1_headers_store,
            owner_account,
        }
    }

    pub async fn store_state_root(
        &self,
        block_number: u64,
        state_root: String,
    ) -> Result<InvokeTransactionResult, HandlerError> {
        let (state_root_high, state_root_low) = get_high_and_low(state_root);

        let entry_point_selector = get_selector_from_name("store_state_root")?;

        // Convert block_number to Felt directly
        let block_number_felt = Felt::from(block_number);

        // Convert state_root parts to Felt
        let state_root_low_felt = Felt::from_bytes_be_slice(&state_root_low.to_be_bytes());
        let state_root_high_felt = Felt::from_bytes_be_slice(&state_root_high.to_be_bytes());

        let calldata = vec![block_number_felt, state_root_low_felt, state_root_high_felt];

        self.invoke(entry_point_selector, calldata).await
    }

    pub async fn get_state_root(&self, block_number: u64) -> Result<Vec<Felt>, HandlerError> {
        let entry_point_selector = get_selector_from_name("get_state_root")?;
        let calldata = vec![Felt::from_dec_str(block_number.to_string().as_str())
            .map_err(FieldElementParseError::FromStrError)?];
        self.call(entry_point_selector, calldata).await
    }

    async fn call(
        &self,
        entry_point_selector: Felt,
        calldata: Vec<Felt>,
    ) -> Result<Vec<Felt>, HandlerError> {
        self.provider
            .call(
                FunctionCall {
                    contract_address: self.l1_headers_store,
                    entry_point_selector,
                    calldata,
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .map_err(HandlerError::ProviderError)
    }

    async fn invoke(
        &self,
        entry_point_selector: Felt,
        calldata: Vec<Felt>,
    ) -> Result<InvokeTransactionResult, HandlerError> {
        let chain_id = self.provider.chain_id().await?;
        let account = SingleOwnerAccount::new(
            &self.provider,
            &self.signer,
            self.owner_account,
            chain_id,
            ExecutionEncoding::New,
        );

        account
            .execute_v1(vec![Call {
                to: self.l1_headers_store,
                selector: entry_point_selector,
                calldata,
            }])
            .max_fee(felt!("1000000000000000000"))
            .send()
            .await
            .map_err(HandlerError::AccountError)
    }
}

use starknet::{
    accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount},
    core::{
        types::{BlockId, BlockTag, FieldElement, FunctionCall, InvokeTransactionResult},
        utils::get_selector_from_name,
    },
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
    l1_headers_store: FieldElement,
    owner_account: FieldElement,
}

impl L1HeadersStore {
    pub fn new(
        rpc: &str,
        l1_headers_store: FieldElement,
        signer: LocalWallet,
        owner_account: FieldElement,
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

        let entry_point_selector = get_selector_from_name("store_state_root").unwrap();
        let calldata = vec![
            FieldElement::from_dec_str(block_number.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
            FieldElement::from_byte_slice_be(&state_root_low.to_be_bytes())
                .map_err(FieldElementParseError::FromByteSliceError)?,
            FieldElement::from_byte_slice_be(&state_root_high.to_be_bytes())
                .map_err(FieldElementParseError::FromByteSliceError)?,
        ];

        self.invoke(entry_point_selector, calldata).await
    }

    pub async fn get_state_root(
        &self,
        block_number: u64,
    ) -> Result<Vec<FieldElement>, HandlerError> {
        let entry_point_selector = get_selector_from_name("get_state_root")?;
        let calldata = vec![
            FieldElement::from_dec_str(block_number.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
        ];
        self.call(entry_point_selector, calldata).await
    }

    async fn call(
        &self,
        entry_point_selector: FieldElement,
        calldata: Vec<FieldElement>,
    ) -> Result<Vec<FieldElement>, HandlerError> {
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
        entry_point_selector: FieldElement,
        calldata: Vec<FieldElement>,
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
            .execute(vec![Call {
                to: self.l1_headers_store,
                selector: entry_point_selector,
                calldata,
            }])
            .send()
            .await
            .map_err(HandlerError::AccountError)
    }
}

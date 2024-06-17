use starknet::{
    accounts::{
        single_owner::SignError, Account, AccountError, Call, ExecutionEncoding, SingleOwnerAccount,
    },
    core::{
        types::{BlockId, BlockTag, FieldElement, FunctionCall, InvokeTransactionResult},
        utils::get_selector_from_name,
    },
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, ProviderError, Url},
    signers::LocalWallet,
};

use crate::util::get_high_and_low;

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

    pub async fn store_state_root(&self, block_number: u64, state_root: String) {
        let (state_root_high, state_root_low) = get_high_and_low(state_root);

        let entry_point_selector = get_selector_from_name("store_state_root").unwrap();
        let calldata = vec![
            FieldElement::from_dec_str(block_number.to_string().as_str()).unwrap(),
            FieldElement::from_byte_slice_be(&state_root_low.to_be_bytes()).unwrap(),
            FieldElement::from_byte_slice_be(&state_root_high.to_be_bytes()).unwrap(),
        ];

        match self.invoke(entry_point_selector, calldata).await {
            Ok(_) => (),
            Err(e) => tracing::error!("{:?}", e),
        }
    }

    pub async fn get_state_root(
        &self,
        block_number: u64,
    ) -> Result<Vec<FieldElement>, ProviderError> {
        let entry_point_selector = get_selector_from_name("get_state_root").unwrap();
        let calldata = vec![FieldElement::from_dec_str(block_number.to_string().as_str()).unwrap()];
        self.call(entry_point_selector, calldata).await
    }

    async fn call(
        &self,
        entry_point_selector: FieldElement,
        calldata: Vec<FieldElement>,
    ) -> Result<Vec<FieldElement>, ProviderError> {
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
    }

    async fn invoke(
        &self,
        entry_point_selector: FieldElement,
        calldata: Vec<FieldElement>,
    ) -> Result<
        InvokeTransactionResult,
        AccountError<SignError<starknet::signers::local_wallet::SignError>>,
    > {
        let chain_id = self.provider.chain_id().await.unwrap();
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
    }
}

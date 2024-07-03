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

/// The `L1HeadersStore` struct is responsible for interacting with a smart contract
/// that stores L1 state roots on the StarkNet blockchain.
pub struct L1HeadersStore {
    provider: JsonRpcClient<HttpTransport>,
    signer: LocalWallet,
    l1_headers_store: FieldElement,
    owner_account: FieldElement,
}

impl L1HeadersStore {
    /// Creates a new instance of `L1HeadersStore`.
    ///
    /// # Arguments
    ///
    /// * `rpc` - A string slice that holds the URL of the JSON-RPC endpoint.
    /// * `l1_headers_store` - The field element representing the L1 headers store contract address.
    /// * `signer` - The local wallet used for signing transactions.
    /// * `owner_account` - The field element representing the owner's account address.
    ///
    /// # Returns
    ///
    /// A new instance of `L1HeadersStore`.
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

    /// Sends a transaction to the L1 headers store contract to store a state root.
    ///
    /// # Arguments
    ///
    /// * `block_number` - The block number associated with the state root.
    /// * `state_root` - The state root as a string.
    ///
    /// # Returns
    ///
    /// A result containing the invocation transaction result or a handler error.
    pub async fn store_state_root(
        &self,
        block_number: u64,
        state_root: String,
    ) -> Result<InvokeTransactionResult, HandlerError> {
        let (state_root_high, state_root_low) = get_high_and_low(state_root);

        let entry_point_selector = get_selector_from_name("store_state_root")?;
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

    /// Calls the L1 headers store contract to get the state root for a specific block number.
    ///
    /// # Arguments
    ///
    /// * `block_number` - The block number for which the state root is requested.
    ///
    /// # Returns
    ///
    /// A result containing a vector of field elements representing the state root or a handler error.
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

    /// Sends a call to the L1 headers store contract.
    ///
    /// # Arguments
    ///
    /// * `entry_point_selector` - The entry point selector of the contract function.
    /// * `calldata` - The calldata to be sent to the contract function.
    ///
    /// # Returns
    ///
    /// A result containing a vector of field elements representing the call result or a handler error.
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

    /// Sends an invocation transaction to the L1 headers store contract.
    ///
    /// # Arguments
    ///
    /// * `entry_point_selector` - The entry point selector of the contract function.
    /// * `calldata` - The calldata to be sent to the contract function.
    ///
    /// # Returns
    ///
    /// A result containing the invocation transaction result or a handler error.
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

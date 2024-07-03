use primitive_types::U256;
use proof_generator::model::{account_proof::AccountProof, storage_proof::StorageProof};
use starknet::{
    accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount},
    core::{
        types::{BlockId, BlockTag, FieldElement, FunctionCall, InvokeTransactionResult},
        utils::get_selector_from_name,
    },
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, Url,
    },
    signers::LocalWallet,
};

use crate::{
    error::{FieldElementParseError, HandlerError},
    util::{get_high_and_low, prepare_array_data},
};

/// The `FactRegistry` struct is responsible for interacting with a smart contract
/// that serves as a registry for various proofs (e.g., storage proofs, account proofs).
/// It communicates with a StarkNet blockchain via JSON-RPC.
pub struct FactRegistry {
    provider: JsonRpcClient<HttpTransport>,
    signer: LocalWallet,
    fact_registry: FieldElement,
    owner_account: FieldElement,
}

#[allow(dead_code)]
impl FactRegistry {
    /// Creates a new instance of `FactRegistry`.
    ///
    /// # Arguments
    ///
    /// * `rpc` - A string slice that holds the URL of the JSON-RPC endpoint.
    /// * `fact_registry` - The field element representing the fact registry contract address.
    /// * `signer` - The local wallet used for signing transactions.
    /// * `owner_account` - The field element representing the owner's account address.
    ///
    /// # Returns
    ///
    /// A new instance of `FactRegistry`.
    pub fn new(
        rpc: &str,
        fact_registry: FieldElement,
        signer: LocalWallet,
        owner_account: FieldElement,
    ) -> Self {
        let url = Url::parse(rpc).unwrap();
        let provider = JsonRpcClient::new(HttpTransport::new(url));

        Self {
            provider,
            signer,
            fact_registry,
            owner_account,
        }
    }

    /// Sends a transaction to the fact registry contract to prove storage.
    ///
    /// # Arguments
    ///
    /// * `block_number` - The block number at which the proof is valid.
    /// * `account_address` - The U256 representation of the account address.
    /// * `storage_proof` - The storage proof data.
    /// * `slot` - The storage slot as a string.
    ///
    /// # Returns
    ///
    /// A result containing the invocation transaction result or a handler error.
    pub async fn prove_storage(
        &self,
        block_number: u64,
        account_address: U256,
        storage_proof: StorageProof,
        slot: String,
    ) -> Result<InvokeTransactionResult, HandlerError> {
        let (slot_high, slot_low) = get_high_and_low(slot.clone());

        let (bytes_len, mut bytes) = prepare_array_data(storage_proof.bytes)?;
        let (data_len, mut data) = prepare_array_data(storage_proof.data)?;

        let entry_point_selector = get_selector_from_name("prove_storage")?;
        let mut calldata = vec![
            FieldElement::from_dec_str(block_number.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
            FieldElement::from_dec_str(account_address.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
            FieldElement::from_byte_slice_be(&slot_low.to_be_bytes())
                .map_err(FieldElementParseError::FromByteSliceError)?,
            FieldElement::from_byte_slice_be(&slot_high.to_be_bytes())
                .map_err(FieldElementParseError::FromByteSliceError)?,
        ];

        calldata.push(bytes_len);
        calldata.append(&mut bytes);
        calldata.push(data_len);
        calldata.append(&mut data);

        self.invoke(entry_point_selector, calldata).await
    }

    /// Sends a transaction to the fact registry contract to prove an account.
    ///
    /// # Arguments
    ///
    /// * `block_number` - The block number at which the proof is valid.
    /// * `account_proof` - The account proof data.
    ///
    /// # Returns
    ///
    /// A result containing the invocation transaction result or a handler error.
    pub async fn prove_account(
        &self,
        block_number: u64,
        account_proof: AccountProof,
    ) -> Result<InvokeTransactionResult, HandlerError> {
        let (bytes_len, mut bytes) = prepare_array_data(account_proof.bytes)?;
        let (data_len, mut data) = prepare_array_data(account_proof.data)?;

        let entry_point_selector = get_selector_from_name("prove_account")?;
        let mut calldata = vec![
            FieldElement::from_dec_str(4.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
            FieldElement::from_dec_str(account_proof.address.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
            FieldElement::from_dec_str(block_number.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
        ];
        calldata.push(bytes_len);
        calldata.append(&mut bytes);
        calldata.push(data_len);
        calldata.append(&mut data);

        self.invoke(entry_point_selector, calldata).await
    }

    /// Calls the fact registry contract to get storage data.
    ///
    /// # Arguments
    ///
    /// * `block_number` - The block number at which the data is valid.
    /// * `account_address` - The U256 representation of the account address.
    /// * `slot` - The storage slot as a string.
    ///
    /// # Returns
    ///
    /// A result containing a vector of field elements representing the storage data or a handler error.
    pub async fn get_storage(
        &self,
        block_number: u64,
        account_address: U256,
        slot: String,
    ) -> Result<Vec<FieldElement>, HandlerError> {
        let (slot_high, slot_low) = get_high_and_low(slot.clone());
        let entry_point_selector = get_selector_from_name("get_storage")?;
        let calldata = vec![
            FieldElement::from_dec_str(block_number.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
            FieldElement::from_dec_str(account_address.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
            FieldElement::from_byte_slice_be(&slot_low.to_be_bytes())
                .map_err(FieldElementParseError::FromByteSliceError)?,
            FieldElement::from_byte_slice_be(&slot_high.to_be_bytes())
                .map_err(FieldElementParseError::FromByteSliceError)?,
        ];
        self.call(entry_point_selector, calldata).await
    }

    /// Calls the fact registry contract to get the verified account hash.
    ///
    /// # Arguments
    ///
    /// * `block_number` - The block number at which the data is valid.
    /// * `account_address` - The U256 representation of the account address.
    ///
    /// # Returns
    ///
    /// A result containing a vector of field elements representing the account hash or a handler error.
    pub async fn get_verified_account_hash(
        &self,
        block_number: u64,
        account_address: U256,
    ) -> Result<Vec<FieldElement>, HandlerError> {
        let entry_point_selector = get_selector_from_name("get_verified_account_hash")?;
        let calldata = vec![
            FieldElement::from_dec_str(block_number.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
            FieldElement::from_dec_str(account_address.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
        ];

        self.call(entry_point_selector, calldata).await
    }

    /// Sends a call to the fact registry contract.
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
                    contract_address: self.fact_registry,
                    entry_point_selector,
                    calldata,
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .map_err(HandlerError::ProviderError)
    }

    /// Sends an invocation transaction to the fact registry contract.
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
                to: self.fact_registry,
                selector: entry_point_selector,
                calldata,
            }])
            .send()
            .await
            .map_err(HandlerError::AccountError)
    }
}

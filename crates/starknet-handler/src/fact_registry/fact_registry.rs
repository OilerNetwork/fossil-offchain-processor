use primitive_types::U256;
use proof_generator::model::{account_proof::AccountProof, storage_proof::StorageProof};
use starknet::{
    accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount},
    core::{
        types::{BlockId, BlockTag, Felt, FunctionCall, InvokeTransactionResult},
        utils::get_selector_from_name,
    },
    macros::felt,
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

pub struct FactRegistry {
    provider: JsonRpcClient<HttpTransport>,
    signer: LocalWallet,
    fact_registry: Felt,
    owner_account: Felt,
}

#[allow(dead_code)]
impl FactRegistry {
    pub fn new(rpc: &str, fact_registry: Felt, signer: LocalWallet, owner_account: Felt) -> Self {
        let url = Url::parse(rpc).unwrap();
        println!("eth rpc url: {:?}", url);
        let provider = JsonRpcClient::new(HttpTransport::new(url));

        Self {
            provider,
            signer,
            fact_registry,
            owner_account,
        }
    }

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

        // Convert block_number to Felt directly
        let block_number_felt = Felt::from(block_number);

        // Convert account_address to bytes and then to Felt
        let mut account_address_bytes = [0u8; 32];
        account_address.to_big_endian(&mut account_address_bytes);
        let account_address_felt = Felt::from_bytes_be_slice(&account_address_bytes);

        // Convert slot parts to Felt
        let slot_low_felt = Felt::from_bytes_be_slice(&slot_low.to_be_bytes());
        let slot_high_felt = Felt::from_bytes_be_slice(&slot_high.to_be_bytes());

        let mut calldata = vec![
            block_number_felt,
            account_address_felt,
            slot_low_felt,
            slot_high_felt,
        ];

        calldata.push(bytes_len);
        calldata.append(&mut bytes);
        calldata.push(data_len);
        calldata.append(&mut data);

        self.invoke(entry_point_selector, calldata).await
    }

    pub async fn prove_account(
        &self,
        block_number: u64,
        account_proof: AccountProof,
    ) -> Result<InvokeTransactionResult, HandlerError> {
        let (bytes_len, mut bytes) = prepare_array_data(account_proof.bytes)?;
        let (data_len, mut data) = prepare_array_data(account_proof.data)?;

        let entry_point_selector = get_selector_from_name("prove_account")?;
        let mut calldata = vec![
            Felt::from_dec_str(0.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
            Felt::from_dec_str(account_proof.address.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
            Felt::from_dec_str(block_number.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
        ];
        calldata.push(bytes_len);
        calldata.append(&mut bytes);
        calldata.push(data_len);
        calldata.append(&mut data);

        self.invoke(entry_point_selector, calldata).await
    }

    pub async fn get_storage(
        &self,
        block_number: u64,
        account_address: U256,
        slot: String,
    ) -> Result<Vec<Felt>, HandlerError> {
        let (slot_high, slot_low) = get_high_and_low(slot.clone());
        let entry_point_selector = get_selector_from_name("get_storage")?;

        // Convert block_number to Felt directly
        let block_number_felt = Felt::from(block_number);

        // Convert account_address to bytes and then to Felt
        let mut account_address_bytes = [0u8; 32];
        account_address.to_big_endian(&mut account_address_bytes);
        let account_address_felt = Felt::from_bytes_be_slice(&account_address_bytes);

        // Convert slot parts to Felt
        let slot_low_felt = Felt::from_bytes_be_slice(&slot_low.to_be_bytes());
        let slot_high_felt = Felt::from_bytes_be_slice(&slot_high.to_be_bytes());

        let calldata = vec![
            block_number_felt,
            account_address_felt,
            slot_low_felt,
            slot_high_felt,
        ];

        self.call(entry_point_selector, calldata).await
    }

    pub async fn get_verified_account_hash(
        &self,
        block_number: u64,
        account_address: U256,
    ) -> Result<Vec<Felt>, HandlerError> {
        let entry_point_selector = get_selector_from_name("get_verified_account_hash")?;
        let calldata = vec![
            Felt::from_dec_str(block_number.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
            Felt::from_dec_str(account_address.to_string().as_str())
                .map_err(FieldElementParseError::FromStrError)?,
        ];

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
                    contract_address: self.fact_registry,
                    entry_point_selector,
                    calldata,
                },
                BlockId::Tag(BlockTag::Pending),
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
        let mut account = SingleOwnerAccount::new(
            &self.provider,
            &self.signer,
            self.owner_account,
            chain_id,
            ExecutionEncoding::New,
        );
        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        // let nonce = self
        //     .provider
        //     .get_nonce((BlockId::Tag(BlockTag::Latest)), self.fact_registry)
        //     .await
        //     .map_err(HandlerError::ProviderError)?;
        //
        account
            .execute_v1(vec![Call {
                to: self.fact_registry,
                selector: entry_point_selector,
                calldata,
            }])
            .max_fee(felt!("1000000000000000000"))
            .send()
            .await
            .map_err(HandlerError::AccountError)
    }
}

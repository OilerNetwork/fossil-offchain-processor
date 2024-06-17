use proof_generator::model::account_proof::AccountProof;
use starknet::{
    accounts::{
        single_owner::SignError, Account, AccountError, Call, ExecutionEncoding, SingleOwnerAccount,
    },
    core::{
        types::{BlockId, BlockTag, FieldElement, FunctionCall, InvokeTransactionResult},
        utils::get_selector_from_name,
    },
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, ProviderError, Url,
    },
    signers::LocalWallet,
};

use crate::util::get_high_and_low;

pub struct FactRegistry {
    provider: JsonRpcClient<HttpTransport>,
    signer: LocalWallet,
    fact_registry: FieldElement,
    owner_account: FieldElement,
}

#[allow(dead_code)]
impl FactRegistry {
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

    pub async fn prove_storage(
        &self,
        block_number: u64,
        account_proof: AccountProof,
        slot: String,
    ) {
        let (slot_high, slot_low) = get_high_and_low(slot.clone());
        let bytes_len =
            FieldElement::from_dec_str(account_proof.bytes.len().to_string().as_str()).unwrap();
        let data_len =
            FieldElement::from_dec_str(account_proof.data.len().to_string().as_str()).unwrap();

        let mut bytes = account_proof
            .bytes
            .iter()
            .map(|b| FieldElement::from_dec_str(b.to_string().as_str()).unwrap())
            .collect::<Vec<_>>();

        let mut data = account_proof
            .data
            .iter()
            .map(|b| FieldElement::from_dec_str(b.to_string().as_str()).unwrap())
            .collect::<Vec<_>>();

        let entry_point_selector = get_selector_from_name("prove_storage").unwrap();
        let mut calldata = vec![
            FieldElement::from_dec_str(block_number.to_string().as_str()).unwrap(),
            FieldElement::from_dec_str(account_proof.address.to_string().as_str()).unwrap(),
            FieldElement::from_byte_slice_be(&slot_low.to_be_bytes()).unwrap(),
            FieldElement::from_byte_slice_be(&slot_high.to_be_bytes()).unwrap(),
        ];

        calldata.push(bytes_len);
        calldata.append(&mut bytes);
        calldata.push(data_len);
        calldata.append(&mut data);

        match self.invoke(entry_point_selector, calldata).await {
            Ok(_) => (),
            Err(e) => tracing::error!("{:?}", e),
        }
    }

    pub async fn prove_account(&self, block_number: u64, account_proof: AccountProof) {
        let bytes_len =
            FieldElement::from_dec_str(account_proof.bytes.len().to_string().as_str()).unwrap();
        let data_len =
            FieldElement::from_dec_str(account_proof.data.len().to_string().as_str()).unwrap();

        let mut bytes = account_proof
            .bytes
            .iter()
            .map(|b| FieldElement::from_dec_str(b.to_string().as_str()).unwrap())
            .collect::<Vec<_>>();

        let mut data = account_proof
            .data
            .iter()
            .map(|b| FieldElement::from_dec_str(b.to_string().as_str()).unwrap())
            .collect::<Vec<_>>();

        let entry_point_selector = get_selector_from_name("prove_account").unwrap();
        let mut calldata = vec![
            FieldElement::from_dec_str(4.to_string().as_str()).unwrap(),
            FieldElement::from_dec_str(account_proof.address.to_string().as_str()).unwrap(),
            FieldElement::from_dec_str(block_number.to_string().as_str()).unwrap(),
        ];
        calldata.push(bytes_len);
        calldata.append(&mut bytes);
        calldata.push(data_len);
        calldata.append(&mut data);

        match self.invoke(entry_point_selector, calldata).await {
            Ok(_) => (),
            Err(e) => tracing::error!("{:?}", e),
        }
    }

    pub async fn get_storage(
        &self,
        block_number: u64,
        account_proof: AccountProof,
        slot: String,
    ) -> Result<Vec<FieldElement>, ProviderError> {
        let (slot_high, slot_low) = get_high_and_low(slot.clone());
        let entry_point_selector = get_selector_from_name("get_storage").unwrap();
        let calldata = vec![
            FieldElement::from_dec_str(block_number.to_string().as_str()).unwrap(),
            FieldElement::from_dec_str(account_proof.address.to_string().as_str()).unwrap(),
            FieldElement::from_byte_slice_be(&slot_low.to_be_bytes()).unwrap(),
            FieldElement::from_byte_slice_be(&slot_high.to_be_bytes()).unwrap(),
        ];
        self.call(entry_point_selector, calldata).await
    }

    pub async fn get_verified_account_hash(
        &self,
        block_number: u64,
        account_proof: AccountProof,
    ) -> Result<Vec<FieldElement>, ProviderError> {
        let entry_point_selector = get_selector_from_name("get_verified_account_hash").unwrap();
        let calldata = vec![
            FieldElement::from_dec_str(block_number.to_string().as_str()).unwrap(),
            FieldElement::from_dec_str(account_proof.address.to_string().as_str()).unwrap(),
        ];

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
                    contract_address: self.fact_registry,
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
                to: self.fact_registry,
                selector: entry_point_selector,
                calldata,
            }])
            .send()
            .await
    }
}

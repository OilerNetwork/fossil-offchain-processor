use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use dotenv::dotenv;
use reqwest::StatusCode;
use serde::Deserialize;
use std::fmt::Write;
use std::str::FromStr;
use std::vec;

use crate::state::AppState;
use proof_generator::{
    controller::mev_blocker::call_mev_blocker_api,
    model::{
        account_proof::AccountProof, eth_rpc::Input, proof::Proof, storage_proof::StorageProof,
    },
};

use starknet::{
    core::types::FieldElement,
    signers::{LocalWallet, SigningKey},
};
use starknet_handler::{
    fact_registry::fact_registry::FactRegistry, l1_headers_store::l1_headers_store::L1HeadersStore,
};

#[derive(Deserialize, Clone)]
pub struct StorageRequest {
    pub block_number: u64,
    pub account_proof: AccountProof,
    pub slot: String,
    //
    pub trie_proof: serde_json::Value,
    pub storage_keys: Vec<String>,
}

pub async fn get_storage_value(
    State(app_state): State<AppState>,
    Json(input): Json<StorageRequest>,
) -> impl IntoResponse {
    dotenv().ok();

    let private_key = dotenv::var("KATANA_8_PRIVATE_KEY").unwrap();
    let owner_account = dotenv::var("KATANA_8_ADDRESS").unwrap();
    let owner_account = FieldElement::from_str(owner_account.as_str()).unwrap();
    let signer = LocalWallet::from(SigningKey::from_secret_scalar(
        FieldElement::from_hex_be(&private_key).unwrap(),
    ));
    let fact_registry_address =
        FieldElement::from_hex_be(dotenv::var("FACT_REGISTRY_ADDRESS").unwrap().as_str()).unwrap();
    let l1_headers_store_address =
        FieldElement::from_hex_be(dotenv::var("L1_HEADERS_STORE_ADDRESS").unwrap().as_str())
            .unwrap();

    let fact_registry_contract = FactRegistry::new(
        "http://localhost:5050",
        fact_registry_address,
        signer.clone(),
        owner_account,
    );
    let l1_headers_store_contract = L1HeadersStore::new(
        "http://localhost:5051",
        l1_headers_store_address,
        signer,
        owner_account,
    );

    // 1. request storage
    let response_storage = fact_registry_contract
        .get_storage(
            input.block_number, //
            input.account_proof.clone(),
            input.slot.clone(),
        )
        .await;

    let response_storage = match response_storage {
        Ok(res) => {
            tracing::info!("Result response_storage: {:?}", res.len());
            res
        }
        Err(err) => {
            tracing::error!("Error response_storage: {:?}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Error, respond_storage"),
            )
                .into_response();
        }
    };

    // If the storage is already available
    if !response_storage.is_empty() {
        let mut result_string = String::new();
        for field_element in &response_storage {
            let bytes = field_element.to_bytes_be();
            for byte in bytes {
                write!(&mut result_string, "{:02x}", byte).unwrap();
            }
        }
        return (StatusCode::OK, Json(&result_string)).into_response();
    } else {
        // 2. request storage proof by calling `call_mev_blocker_api` in proof-generator
        let api_input = Input {
            account_address: input.account_proof.address.to_string(),
            storage_keys: input.storage_keys.clone(),
        };

        // I think the name `call_mev_blocker_api` should be changed
        let response = call_mev_blocker_api(State(app_state.client.clone()), Json(api_input))
            .await
            .into_response();

        // TODO let storage_proof = func(response) ...

        // 3. request state_root by calling `get_state_root` in l1_headers_store
        let state_root = l1_headers_store_contract
            .get_state_root(
                input.block_number, //
            )
            .await;

        let state_root = match state_root {
            Ok(res) => {
                tracing::info!("Result state_root: {:?}", res.len());
                res
            }
            Err(err) => {
                tracing::error!("Error state_root: {:?}", err);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json("Error, get_state_root"),
                )
                    .into_response();
            }
        };

        // TODO 3-ALT.
        //        request the RLP Encoded Block from the proof_generator
        //        then process the block with the starknet_handler, which will save the state_root

        // TODO 4. request storage proof again
        let api_input = Input {
            account_address: input.account_proof.address.to_string(),
            storage_keys: input.storage_keys,
        };
        let response = call_mev_blocker_api(State(app_state.client), Json(api_input))
            .await
            .into_response();

        // TODO let storage_proof = func(response) ...

        // TODO 5. check if Account Already verified
        // TODO 5-ALT. Account not verified, verify Account proof
        // TODO 5-ALT2. error RESPONSE, Proof INVALID

        // TODO 6. verify storage proof
        // TODO 6-ALT. proof invalid, error RESPONSE

        return (StatusCode::OK, Json("wip")).into_response();
    }
}

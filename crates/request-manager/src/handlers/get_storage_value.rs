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
    tracing::info!("Request storage");
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
        tracing::info!("Request storage proof");
        let storage_proof = {
            let api_input = Input {
                account_address: input.account_proof.address.to_string(),
                storage_keys: input.storage_keys.clone(),
            };

            // I think the name `call_mev_blocker_api` should be changed
            let response = call_mev_blocker_api(State(app_state.client.clone()), Json(api_input))
                .await
                .into_response();

            let bytes = match axum::body::to_bytes(response.into_body(), usize::MAX).await {
                Ok(bytes) => bytes,
                Err(err) => {
                    tracing::error!("Error converting response body to bytes: {:?}", err);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json("Error converting response body to bytes"),
                    )
                        .into_response();
                }
            };

            let storage_proof: StorageProof = match serde_json::from_slice(&bytes) {
                Ok(proof) => proof,
                Err(err) => {
                    tracing::error!("Error deserializing response to StorageProof: {:?}", err);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json("Error deserializing response to StorageProof"),
                    )
                        .into_response();
                }
            };

            storage_proof
        };

        // 3. request state_root by calling `get_state_root` in l1_headers_store
        tracing::info!("Request state_root by calling `get_state_root` in l1_headers_store");
        let _state_root = {
            match l1_headers_store_contract
                .get_state_root(input.block_number)
                .await
            {
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
        };

        // 3-ALT.
        //        request the RLP Encoded Block from the proof_generator
        //        then process the block with the starknet_handler, which will save the state_root
        {

            // let rlp_encoded_block = proof_generator.get_rlp_encoded_block();
            // starknet_handler::interface::process_block(rlp_encoded_block);
        }

        // 4. request storage proof again
        tracing::info!("Request storage proof");
        let storage_proof = {
            let api_input = Input {
                account_address: input.account_proof.address.to_string(),
                storage_keys: input.storage_keys.clone(),
            };

            // I think the name `call_mev_blocker_api` should be changed
            let response = call_mev_blocker_api(State(app_state.client.clone()), Json(api_input))
                .await
                .into_response();

            let bytes = match axum::body::to_bytes(response.into_body(), usize::MAX).await {
                Ok(bytes) => bytes,
                Err(err) => {
                    tracing::error!("Error converting response body to bytes: {:?}", err);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json("Error converting response body to bytes"),
                    )
                        .into_response();
                }
            };

            let storage_proof: StorageProof = match serde_json::from_slice(&bytes) {
                Ok(proof) => proof,
                Err(err) => {
                    tracing::error!("Error deserializing response to StorageProof: {:?}", err);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json("Error deserializing response to StorageProof"),
                    )
                        .into_response();
                }
            };
            storage_proof
        };

        // 5. check if Account Already verified
        {
            tracing::info!("Check if account already verified");
            // let is_verified_account =
            //     starknet_handler_interface.check_account_proof_status(input.account_proof);
            // if is_verified_account == false {
            //     if starknet_handler_interface.verify_account_proof(input.account_proof) == false {
            //         tracing::error!("Error while verifying the account proof: {:?}", err);
            //         return (
            //             StatusCode::INTERNAL_SERVER_ERROR,
            //             Json("Error while verifying the account proof"),
            //         )
            //             .into_response();
            //     } else {
            //         //
            //     }
            // };
        }

        // 6. verify storage proof
        {
            tracing::info!("Verifying the storage proof");
            // let is_verified_storage =
            //     starknet_handler_interface.verify_storage_proof(storage_proof);

            // if is_verified_storage == false {
            //     tracing::error!("Error while verifying the storage proof: {:?}", err);
            //     return (
            //         StatusCode::INTERNAL_SERVER_ERROR,
            //         Json("Error while verifying the storage proof"),
            //     );
            // }
        }

        return (StatusCode::OK, Json("done")).into_response();
    }
}

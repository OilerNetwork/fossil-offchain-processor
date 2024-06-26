use axum::{extract::State, response::IntoResponse, Json};
use dotenv::dotenv;
use reqwest::StatusCode;
use serde::Deserialize;
use std::fmt::Write;
use std::str::FromStr;

use primitive_types::U256;

use crate::state::AppState;
use proof_generator::{
    controller::mev_blocker::call_mev_blocker_api,
    model::{eth_rpc::Input, proof::Proof},
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
    pub account_address: String,
    pub slot: String,
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

    let starknet_rpc = dotenv::var("STARKNET_RPC").unwrap();

    let fact_registry_contract = FactRegistry::new(
        starknet_rpc.as_str(),
        fact_registry_address,
        signer.clone(),
        owner_account,
    );
    let l1_headers_store_contract = L1HeadersStore::new(
        starknet_rpc.as_str(),
        l1_headers_store_address,
        signer,
        owner_account,
    );

    // 1. request storage
    tracing::info!("Request storage");
    let response_storage = fact_registry_contract
        .get_storage(
            input.block_number, //
            U256::from_str(&input.account_address).unwrap(),
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
    }

    // 2. request state_root
    tracing::info!("Request state_root by calling `get_state_root` in l1_headers_store");
    {
        match l1_headers_store_contract
            .get_state_root(input.block_number)
            .await
        {
            Ok(res) => {
                tracing::info!("Result state_root: {:?}", res.len());
                res
            }
            Err(err) => {
                tracing::error!("No state_root available, {}", err);
                // TODO: how to get state root for the block from eth
                todo!()
                // let _state_root: Vec<FieldElement> = todo!();
                // l1_headers_store_contract.store_state_root(input.block_number, todo!());
                // _state_root
            }
        };
    };

    // 3. check account proof status on starknet

    let is_account_proved = fact_registry_contract
        .get_verified_account_hash(
            input.block_number,
            U256::from_str(&input.account_address).unwrap(),
        )
        .await
        .is_ok();

    // I think the name `call_mev_blocker_api` should be changed
    // 4. Call eth_getProof
    tracing::info!("Request eth_getProof");
    let eth_proof = {
        let api_input = Input {
            account_address: input.account_address.clone(),
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

        let proof: Proof = match serde_json::from_slice(&bytes) {
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

        proof
    };
    if !is_account_proved {
        tracing::info!("Account is not verified yet, verifying on Starknet");
        match fact_registry_contract
            .prove_account(input.block_number, eth_proof.account_proof.clone())
            .await
        {
            Ok(_) => (),
            Err(err) => {
                tracing::error!("Error while verifying the account proof: {:?}", err);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json("Error while verifying the account proof"),
                )
                    .into_response();
            }
        }
    }

    tracing::info!("Verifying the storage proof");
    match fact_registry_contract
        .prove_storage(
            input.block_number,
            U256::from_str(&input.account_address).unwrap(),
            eth_proof.storage_proof,
            input.slot,
        )
        .await
    {
        Ok(res) => (StatusCode::OK, Json(&res)).into_response(),
        Err(err) => {
            tracing::error!("Error while verifying the storage proof: {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Error while verifying the storage proof"),
            )
                .into_response()
        }
    }
}

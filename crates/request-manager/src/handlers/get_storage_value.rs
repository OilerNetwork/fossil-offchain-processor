use axum::{extract::State, response::IntoResponse, Json};
use dotenv::dotenv;
use reqwest::StatusCode;
use serde::Deserialize;
use std::str::FromStr;

use primitive_types::U256;

use crate::state::AppState;
use proof_generator::{
    controller::{eth_blocks::call_eth_blocks_api, mev_blocker::call_mev_blocker_api},
    model::{
        eth_rpc::{BlockNumber, Input},
        hex::HexString,
        proof::Proof,
    },
};

use starknet::{
    core::types::Felt,
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
    let private_key_felt = Felt::from_hex(&private_key).unwrap();

    let owner_account = dotenv::var("KATANA_8_ADDRESS").unwrap();
    let owner_account = Felt::from_hex(&owner_account).unwrap();

    // Ensure the private key is correctly formatted and converted
    let signing_key = SigningKey::from_secret_scalar(private_key_felt);

    let signer = LocalWallet::from(signing_key);

    let fact_registry_address =
        Felt::from_hex_unchecked(dotenv::var("FACT_REGISTRY_ADDRESS").unwrap().as_str());
    let l1_headers_store_address =
        Felt::from_hex_unchecked(dotenv::var("L1_HEADERS_STORE_ADDRESS").unwrap().as_str());

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
            tracing::info!("Result response_storage: {:?}", res);
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
    if response_storage.len() == 2 {
        let mut value = U256::from(0);
        for (i, field_element) in response_storage.iter().enumerate() {
            let big_int = field_element.to_string();
            let big_int = U256::from_dec_str(&big_int).unwrap();
            value += big_int << (i * 128);
        }
        if value != U256::from(1) {
            return (StatusCode::OK, Json(&value)).into_response();
        }
    }

    // 2. request state_root
    tracing::info!("Request state_root by calling `get_state_root` in l1_headers_store");

    match l1_headers_store_contract
        .get_state_root(input.block_number)
        .await
    {
        Ok(res) => {
            tracing::info!("Result state_root: {:?}", res);

            let mut value: U256 = U256::from(0);

            for (i, field_element) in res.iter().enumerate() {
                let big_int = field_element.to_string();
                let big_int = U256::from_dec_str(&big_int).unwrap();
                value += big_int << (i * 128);
            }

            let result_string = value;

            if result_string == U256::from(0) {
                let api_input = BlockNumber {
                    block_number: HexString::new(&format!("0x{:x}", input.block_number)),
                };
                let response =
                    call_eth_blocks_api(State(app_state.client.clone()), Json(api_input))
                        .await
                        .into_response();

                println!("api response: {:?}", response);
                if response.status() == StatusCode::BAD_REQUEST {
                    tracing::error!("Bad request error: {:?}", response);
                    return (
                        StatusCode::BAD_REQUEST,
                        Json("Error: Bad request to external API"),
                    )
                        .into_response();
                }

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

                println!("bytes: {:?}", bytes);

                if bytes.len() < 68 {
                    tracing::error!("Response body too short, expected at least 68 bytes, got {}", bytes.len());
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json("Response body too short"),
                    )
                        .into_response();
                }

                let state_root = match String::from_utf8(bytes[1..67].to_vec()) {
                    Ok(state_root) => state_root,
                    Err(err) => {
                        tracing::error!("Error converting bytes to string: {:?}", err);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json("Error converting bytes to string"),
                        )
                            .into_response();
                    }
                };

                println!("{}", state_root);
                let _ = l1_headers_store_contract
                    .store_state_root(input.block_number, state_root)
                    .await;
            }
        }
        Err(err) => {
            tracing::info!("No state_root available, {}", err);
            let api_input = BlockNumber {
                block_number: HexString::new(&format!("0x{:x}", input.block_number)),
            };
            println!("api_input: {:?}", api_input);
            let response = call_eth_blocks_api(State(app_state.client.clone()), Json(api_input))
                .await
                .into_response();
            println!("response: {:?}", response);

            if response.status() == StatusCode::BAD_REQUEST {
                tracing::error!("Bad request error: {:?}", response);
                return (
                    StatusCode::BAD_REQUEST,
                    Json("Error: Bad request to external API"),
                )
                    .into_response();
            }

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
            println!("bytes: {:?}", bytes);
            let state_root = String::from_utf8(bytes.to_vec()).unwrap();
            let state_root = state_root.trim_matches('"');
            println!("state_root: {:?}", state_root);
            let _ = l1_headers_store_contract
                .store_state_root(input.block_number, state_root.to_string())
                .await;
        }
    };

    // 3. check account proof status on starknet

    let is_account_proved = fact_registry_contract
        .get_verified_account_hash(
            input.block_number,
            U256::from_str(&input.account_address).unwrap(),
        )
        .await
        .is_ok();

    // 4. Call eth_getProof
    tracing::info!("Request eth_getProof");
    let eth_proof = {
        let api_input = Input {
            account_address: input.account_address.clone(),
            storage_keys: input.storage_keys.clone(),
        };

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
                tracing::error!("Error deserializing response to Proof: {:?}", err);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json("Error deserializing response to Proof"),
                )
                    .into_response();
            }
        };

        proof
    };

    println!("eth proof: {:?}", eth_proof);

    if !is_account_proved {
        tracing::info!("Account is not verified yet, verifying on Starknet");
        println!("block_number: {:?}", input.block_number);
        println!("account_proof: {:?}", eth_proof.account_proof.clone());
        match fact_registry_contract
            .prove_account(input.block_number, eth_proof.account_proof.clone())
            .await
        {
            Ok(res) => {
                let value = res.transaction_hash.to_string();
                match U256::from_dec_str(&value) {
                    Ok(res) => {
                        if res == U256::from(1) {
                            tracing::info!("Account is verified on Starknet");
                        } else if res == U256::from(0) {
                            tracing::info!("Account is not verified on Starknet");
                        } else {
                            tracing::error!(
                                "Starknet returned an error while verifying the account proof"
                            );
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(
                                    "Starknet returned an error while verifying the account proof",
                                ),
                            )
                                .into_response();
                        }
                    }
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
            Err(err) => {
                tracing::error!("Error while verifying the account proof: {:?}", err);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json("Error while calling prove_account"),
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

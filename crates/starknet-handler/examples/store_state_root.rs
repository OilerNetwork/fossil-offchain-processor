mod prove_storage;

use dotenv::dotenv;
use std::str::FromStr;

use starknet::{
    core::types::FieldElement,
    signers::{LocalWallet, SigningKey},
};
use starknet_handler::l1_headers_store::l1_headers_store::L1HeadersStore;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let private_key = dotenv::var("KATANA_8_PRIVATE_KEY").unwrap();

    let owner_account = dotenv::var("KATANA_8_ADDRESS").unwrap();
    let owner_account = FieldElement::from_str(owner_account.as_str()).unwrap();

    let signer = LocalWallet::from(SigningKey::from_secret_scalar(
        FieldElement::from_hex_be(&private_key).unwrap(),
    ));

    let l1_headers_store =
        FieldElement::from_hex_be(dotenv::var("L1_HEADERS_STORE_ADDRESS").unwrap().as_str())
            .unwrap();

    let state_root = "0x07394cbe418daa16e42b87ba67372d4ab4a5df0b05c6e554d158458ce245bc10";

    // NOTE: change block number once its stored
    let block_number = 20;

    let contract = L1HeadersStore::new(
        "http://localhost:5050",
        l1_headers_store,
        signer,
        owner_account,
    );

    let _ = contract
        .store_state_root(block_number, state_root.to_string())
        .await;
}

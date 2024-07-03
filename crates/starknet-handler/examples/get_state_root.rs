use dotenv::{dotenv, var};
use starknet::{
    core::types::Felt,
    signers::{LocalWallet, SigningKey},
};
use starknet_handler::l1_headers_store::l1_headers_store::L1HeadersStore;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let private_key = var("KATANA_8_PRIVATE_KEY").unwrap();

    let owner_account = var("KATANA_8_ADDRESS").unwrap();
    let owner_account = Felt::from_hex_unchecked(owner_account.as_str());

    let signer = LocalWallet::from(SigningKey::from_secret_scalar(Felt::from_hex_unchecked(
        &private_key,
    )));

    let l1_headers_store =
        Felt::from_hex_unchecked(var("L1_HEADERS_STORE_ADDRESS").unwrap().as_str());

    // NOTE: change block number once its stored
    let block_number = 20;

    let contract = L1HeadersStore::new(
        "http://localhost:5050",
        l1_headers_store,
        signer,
        owner_account,
    );
    let res = contract.get_state_root(block_number).await;

    println!("{:?}", res);
}

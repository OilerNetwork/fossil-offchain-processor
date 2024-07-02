mod prove_storage;

use dotenv::dotenv;
use std::str::FromStr;

use starknet::{
    accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount},
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Url,
    },
};
use starknet::{
    core::{
        types::{BlockId, BlockTag, Felt, U256},
        utils::get_selector_from_name,
    },
    macros::felt,
    signers::{LocalWallet, SigningKey},
};
// use starknet::signers::Signer;

/// Convert a hex string to U256
fn u256_from_hex(hex: &str) -> Result<U256, &'static str> {
    let hex = hex.trim_start_matches("0x");
    let bytes = hex::decode(hex).map_err(|_| "Failed to decode hex")?;
    if bytes.len() > 32 {
        return Err("Hex string is too long for U256");
    }
    let mut padded_bytes = [0u8; 32];
    padded_bytes[(32 - bytes.len())..].copy_from_slice(&bytes);
    let high = u128::from_be_bytes(padded_bytes[0..16].try_into().unwrap());
    let low = u128::from_be_bytes(padded_bytes[16..32].try_into().unwrap());
    Ok(U256::from_words(low, high))
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let private_key = dotenv::var("KATANA_8_PRIVATE_KEY").unwrap();

    let owner_account = dotenv::var("KATANA_8_ADDRESS").unwrap();
    let address = Felt::from_str(owner_account.as_str()).unwrap();

    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse("http://localhost:5050").unwrap(),
    ));

    let signer = LocalWallet::from_signing_key(SigningKey::from_secret_scalar(
        Felt::from_hex(&private_key).unwrap(),
    ));

    let mut account = SingleOwnerAccount::new(
        provider,
        signer,
        address,
        Felt::from_hex("0x4b4154414e41").unwrap(),
        ExecutionEncoding::New,
    );
    account.set_block_id(BlockId::Tag(BlockTag::Pending));

    let l1_headers_store_address =
        Felt::from_hex(&dotenv::var("L1_HEADERS_STORE_ADDRESS").unwrap()).unwrap();

    let state_root =
        u256_from_hex("0x07394cbe418daa16e42b87ba67372d4ab4a5df0b05c6e554d158458ce245bc10")
            .unwrap();

    let calls = vec![Call {
        to: l1_headers_store_address,
        selector: get_selector_from_name("store_state_root").unwrap(),
        calldata: vec![
            felt!("22"),
            Felt::from(state_root.low()),
            Felt::from(state_root.high()),
        ],
    }];

    // Create an ExecutionV1 object and set the max_fee
    let execution = account
        .execute_v1(calls)
        .max_fee(felt!("1000000000000000000")); // Example max_fee value

    let result = execution.send().await;

    match result {
        Ok(res) => println!("Transaction hash: {}", res.transaction_hash),
        Err(e) => eprintln!("Failed to send transaction: {:?}", e),
    }
}

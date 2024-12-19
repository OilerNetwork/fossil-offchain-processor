use std::env;

use dotenv::dotenv;
use eyre::Result;
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    core::{chain_id, types::Call, types::U256, utils::get_selector_from_name},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Url},
    signers::{LocalWallet, SigningKey},
};
use starknet_crypto::Felt;

pub const PITCH_LAKE_V1: &str = "0x50495443485f4c414b455f5631";
pub const DEVNET_JUNO_CHAIN_ID: &str = "0x534e5f4a554e4f5f53455155454e434552";

#[derive(Debug)]
pub struct JobRequest {
    pub vault_address: Felt,
    pub timestamp: u64,
    pub program_id: Felt, // 'PITCH_LAKE_V1'
}

#[derive(Debug)]
pub struct PitchLakeResult {
    pub twap: U256,
    pub volatility: u128,
    pub reserve_price: U256,
}

#[derive(Debug)]
pub struct FossilStarknetAccount {
    pub account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
}

impl Default for FossilStarknetAccount {
    fn default() -> Self {
        Self::new()
    }
}

impl FossilStarknetAccount {
    pub fn new() -> Self {
        dotenv().ok();

        let rpc_url =
            env::var("STARKNET_RPC_URL").expect("STARKNET_RPC_URL should be provided as env vars.");
        let account_private_key = env::var("STARKNET_PRIVATE_KEY")
            .expect("STARKNET_PRIVATE_KEY should be provided as env vars.");
        let account_address = env::var("STARKNET_ACCOUNT_ADDRESS")
            .expect("STARKNET_ACCOUNT_ADDRESS should be provided as env vars.");
        let network = env::var("NETWORK").expect("NETWORK should be provided as env vars.");

        let chain_id = match network.as_str() {
            "MAINNET" => chain_id::MAINNET,
            "SEPOLIA" => chain_id::SEPOLIA,
            "DEVNET_KATANA" => chain_id::SEPOLIA,
            "DEVNET_JUNO" => Felt::from_hex(DEVNET_JUNO_CHAIN_ID).unwrap(),
            _ => panic!("Invalid network provided. Must be one of: MAINNET, SEPOLIA, DEVNET_KATANA, DEVNET_JUNO"),
        };

        let provider = JsonRpcClient::new(HttpTransport::new(
            Url::parse(&rpc_url).expect("Invalid rpc url provided"),
        ));

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            Felt::from_hex(&account_private_key).expect("Invalid private key provided"),
        ));

        Self {
            account: SingleOwnerAccount::new(
                provider,
                signer,
                Felt::from_hex(&account_address).expect("Invalid address provided"),
                chain_id,
                ExecutionEncoding::New,
            ),
        }
    }

    pub async fn callback_to_contract(
        &self,
        client_address: Felt,
        job_request: &JobRequest,
        result: &PitchLakeResult,
    ) -> Result<Felt> {
        let calldata = format_pitchlake_calldata(job_request, result);
        let tx = self
            .account
            .execute_v1(vec![Call {
                selector: get_selector_from_name("fossil_callback").unwrap(),
                calldata,
                to: client_address,
            }])
            .send()
            .await?;

        Ok(tx.transaction_hash)
    }
}

pub fn format_pitchlake_calldata(
    job_request: &JobRequest,
    pitch_lake_result: &PitchLakeResult,
) -> Vec<Felt> {
    let mut calldata = Vec::new();

    // Serialize JobRequest into Felt values
    let job_request_felts = vec![
        job_request.vault_address,
        Felt::from(job_request.timestamp),
        job_request.program_id,
    ];

    // Prepend JobRequest length
    calldata.push(Felt::from(job_request_felts.len() as u64));
    calldata.extend(job_request_felts);

    // Serialize PitchLakeResult into Felt values
    let pitch_lake_result_felts = vec![
        Felt::from(pitch_lake_result.twap.low()),
        Felt::from(pitch_lake_result.twap.high()),
        Felt::from(pitch_lake_result.volatility),
        Felt::from(pitch_lake_result.reserve_price.low()),
        Felt::from(pitch_lake_result.reserve_price.high()),
        // Mocked proof data
        Felt::ZERO,
        Felt::ZERO,
    ];

    // Prepend PitchLakeResult length
    calldata.push(Felt::from(pitch_lake_result_felts.len() as u64));
    calldata.extend(pitch_lake_result_felts);

    println!("calldata: {:?}", calldata);

    calldata
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use starknet::core::types::U256;
    use starknet::{
        core::types::{BlockId, BlockTag, FunctionCall},
        macros::selector,
        providers::{
            jsonrpc::{HttpTransport, JsonRpcClient},
            Provider, Url,
        },
    };
    use starknet_crypto::Felt;

    #[ignore]
    #[tokio::test]
    async fn test_callback_to_contract() -> eyre::Result<()> {
        dotenv().ok();

        let rpc_url =
            env::var("STARKNET_RPC_URL").expect("STARKNET_RPC_URL should be provided as env vars.");

        let provider = JsonRpcClient::new(HttpTransport::new(
            Url::parse(&rpc_url).expect("Invalid rpc url provided"),
        ));

        let account = FossilStarknetAccount::default();

        let client_address =
            Felt::from_hex("0x039812d6db47b5bdeafb002fa759e84257607d0b97b7dab04d0cf894dda5c7cb")
                .unwrap();

        let vault_address =
            Felt::from_hex("0x02074629654fa9ce01e19464e7ba6d22527bca28de390012d4705082fba63f4b")
                .unwrap();

        let round_id = 1;
        let round_address = provider
            .call(
                FunctionCall {
                    contract_address: vault_address,
                    entry_point_selector: selector!("get_round_address"),
                    calldata: vec![Felt::from(round_id), Felt::ZERO],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .expect("failed to call contract");

        let deployment_date = provider
            .call(
                FunctionCall {
                    contract_address: round_address[0],
                    entry_point_selector: selector!("get_deployment_date"),
                    calldata: vec![],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .expect("failed to call contract");

        let job_request = JobRequest {
            vault_address,
            timestamp: deployment_date[0].try_into().unwrap(),
            program_id: Felt::from_hex(PITCH_LAKE_V1).unwrap(),
        };

        let pitch_lake_result = PitchLakeResult {
            twap: U256::from(5000_u64),           // Random TWAP value
            volatility: 100,                      // Random volatility value
            reserve_price: U256::from(20000_u64), // Random reserve price value
        };

        // Execute the callback to the contract
        let tx_hash = account
            .callback_to_contract(client_address, &job_request, &pitch_lake_result)
            .await?;

        // Print or assert the transaction hash to verify the function executed successfully
        println!("Transaction Hash: {:?}", tx_hash);

        // Simple assertion to verify the hash isn't zero (indicating a valid transaction)
        assert_ne!(tx_hash, Felt::ZERO);

        Ok(())
    }
}

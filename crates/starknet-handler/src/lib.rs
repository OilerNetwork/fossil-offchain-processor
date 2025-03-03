use std::{env, sync::Arc, time::Duration};

pub mod resilience;
use dotenv::dotenv;
use eyre::{eyre, Result};
use resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    core::{chain_id, types::Call, types::U256, utils::get_selector_from_name},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Url},
    signers::{LocalWallet, SigningKey},
};
use starknet_crypto::Felt;
use tokio::time::sleep;

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
    circuit_breaker: Arc<CircuitBreaker>,
}

impl Default for FossilStarknetAccount {
    fn default() -> Self {
        match Self::new() {
            Ok(account) => account,
            Err(e) => {
                tracing::error!("Error creating default FossilStarknetAccount: {}", e);
                std::process::exit(1); // Exit the program on error
            }
        }
    }
}

impl FossilStarknetAccount {
    pub fn new() -> Result<Self> {
        dotenv().ok();

        let rpc_url = env::var("STARKNET_RPC_URL")
            .map_err(|_| eyre!("STARKNET_RPC_URL should be provided as env vars"))?;

        let account_private_key = env::var("STARKNET_PRIVATE_KEY")
            .map_err(|_| eyre!("STARKNET_PRIVATE_KEY should be provided as env vars"))?;

        let account_address = env::var("STARKNET_ACCOUNT_ADDRESS")
            .map_err(|_| eyre!("STARKNET_ACCOUNT_ADDRESS should be provided as env vars"))?;

        let network =
            env::var("NETWORK").map_err(|_| eyre!("NETWORK should be provided as env vars"))?;

        let chain_id = match network.as_str() {
            "MAINNET" => chain_id::MAINNET,
            "SEPOLIA" => chain_id::SEPOLIA,
            "DEVNET_KATANA" => Felt::from_hex("0x4b4154414e41")?,
            "DEVNET_JUNO" => Felt::from_hex(DEVNET_JUNO_CHAIN_ID)?,
            _ => panic!("Invalid network provided. Must be one of: MAINNET, SEPOLIA, DEVNET_KATANA, DEVNET_JUNO"),
        };

        let url =
            Url::parse(&rpc_url).unwrap_or_else(|e| panic!("Invalid RPC URL provided: {}", e));

        let provider = JsonRpcClient::new(HttpTransport::new(url));

        let private_key = Felt::from_hex(&account_private_key)?;

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key));

        let address = Felt::from_hex(&account_address)?;

        let circuit_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(60),
        }));

        Ok(Self {
            account: SingleOwnerAccount::new(
                provider,
                signer,
                address,
                chain_id,
                ExecutionEncoding::New,
            ),
            circuit_breaker,
        })
    }

    pub async fn callback_to_contract(
        &self,
        client_address: Felt,
        job_request: &JobRequest,
        result: &PitchLakeResult,
    ) -> Result<Felt> {
        const MAX_ATTEMPTS: u32 = 3;
        const BASE_DELAY_MS: u64 = 1000; // 1 second initial delay

        // Create a context string for logging
        let context = format!(
            "client_address={:#064x}, vault_address={:#064x}, timestamp={}, twap={}, volatility={}, reserve_price={}",
            client_address,
            job_request.vault_address,
            job_request.timestamp,
            result.twap,
            result.volatility,
            result.reserve_price
        );

        // Check if circuit breaker is open
        if !self.circuit_breaker.allow_request().await {
            tracing::warn!(
                "Circuit breaker is open, skipping Starknet callback. {}",
                context
            );
            return Err(eyre!(
                "Circuit breaker is open, service temporarily unavailable"
            ));
        }

        tracing::info!("Preparing Starknet callback: {}", context);

        let calldata = format_pitchlake_calldata(job_request, result);
        let selector = get_selector_from_name("fossil_callback")
            .map_err(|e| eyre!("Failed to get selector for fossil_callback: {}", e))?;

        let call = Call {
            selector,
            calldata,
            to: client_address,
        };

        let mut last_error = None;

        for attempt in 1..=MAX_ATTEMPTS {
            tracing::info!(
                "Attempt {} of {} to send transaction to Starknet: {}",
                attempt,
                MAX_ATTEMPTS,
                context
            );

            match self.account.execute_v3(vec![call.clone()]).send().await {
                Ok(tx) => {
                    tracing::info!(
                        "Transaction sent successfully on attempt {}: tx_hash={:#064x}, {}",
                        attempt,
                        tx.transaction_hash,
                        context
                    );
                    // Record success in circuit breaker
                    self.circuit_breaker.on_success().await;
                    return Ok(tx.transaction_hash);
                }
                Err(e) => {
                    let error_msg =
                        format!("Failed to send transaction on attempt {}: {}", attempt, e);
                    tracing::warn!("{}. Context: {}", error_msg, context);
                    last_error = Some(eyre!("{}: {}", error_msg, context));

                    if attempt < MAX_ATTEMPTS {
                        // Exponential backoff: delay = base_delay * 2^(attempt-1)
                        let delay_ms = BASE_DELAY_MS * (1 << (attempt - 1));
                        tracing::info!("Retrying in {} ms. Context: {}", delay_ms, context);
                        sleep(Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        // Record failure in circuit breaker
        self.circuit_breaker.on_failure().await;

        Err(last_error.unwrap_or_else(|| {
            eyre!(
                "Failed to send transaction after {} attempts. Context: {}",
                MAX_ATTEMPTS,
                context
            )
        }))
    }

    // Add a method to manually reset the circuit breaker
    pub async fn reset_circuit_breaker(&self) {
        self.circuit_breaker.reset().await;
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

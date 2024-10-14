use alloy::{providers::ProviderBuilder, sol};
use dotenv::dotenv;
use eyre::Result;
use std::env;

// Codegen from embedded Solidity code and precompiled bytecode.
sol! {
    #[allow(missing_docs)]
    // solc v0.8.26; solc Counter.sol --via-ir --optimize --bin
    #[sol(rpc, bytecode="0x6080604052348015600f57600080fd5b5061011e8061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c80639663f88f14602d575b600080fd5b6033604c565b6040805192835260208301919091520160405180910390f35b6000806064431160b05760405162461bcd60e51b815260206004820152602560248201527f426c6f636b206e756d626572206d75737420626520677265617465722074686160448201526406e203130360dc1b606482015260840160405180910390fd5b60b960644360c2565b92834092509050565b8181038181111560e257634e487b7160e01b600052601160045260246000fd5b9291505056fea2646970667358221220b23547ed5542ead2de6260575deb6fc65775cd0f04fd8e5e88ba243c70e5dcb364736f6c634300081a0033")]
    contract BlockHashFetcher {
        function getBlockHash() external view returns (uint256 blockNumber, bytes32 blockHash) {
            require(block.number > 100, "Block number must be greater than 100");
            blockNumber = block.number - 100;
            blockHash = blockhash(blockNumber);
            return (blockNumber, blockHash);
        }
    }
}

#[allow(dead_code)]
pub async fn get_finalized_block_hash() -> Result<(u64, String)> {
    dotenv().ok();

    tracing::info!("Getting onchain finalized block hash");
    let rpc_url = env::var("ETH_RPC_URL").expect("RPC_URL must be set");
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_anvil_with_wallet_and_config(|anvil| anvil.fork(rpc_url));

    let contract = BlockHashFetcher::deploy(&provider).await?;

    let builder = contract.getBlockHash();
    let result = builder.call().await?;

    let block_number: u64 = result
        .blockNumber
        .try_into()
        .expect("Failed to convert block number to u64");
    let block_hash = result.blockHash.to_string();

    Ok((block_number, block_hash))
}

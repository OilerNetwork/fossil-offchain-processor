use crate::block_header::BlockHeader;
use ethereum_types::{H160, H256, U256};
use rlp::RlpStream;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RpcBlockHeaderGenesisToLondon {
    pub parent_hash: String,
    pub sha3_uncles: String,
    pub miner: String,
    pub state_root: String,
    pub transactions_root: String,
    pub receipts_root: String,
    pub logs_bloom: String,
    pub difficulty: String,
    pub number: String,
    pub gas_limit: String,
    pub gas_used: String,
    pub timestamp: String,
    pub extra_data: String,
    pub mix_hash: String,
    pub nonce: String,
}

#[derive(Debug)]
pub struct BlockHeaderGenesisToLondon {
    pub parent_hash: H256,
    pub ommers_hash: H256,
    pub beneficiary: H160,
    pub state_root: H256,
    pub transactions_root: H256,
    pub receipts_root: H256,
    pub logs_bloom: [u8; 256],
    pub difficulty: U256,
    pub number: U256,
    pub gas_limit: U256,
    pub gas_used: U256,
    pub timestamp: U256,
    pub extra_data: Vec<u8>,
    pub mix_hash: H256,
    pub nonce: [u8; 8],
}

impl BlockHeaderGenesisToLondon {
    pub fn from_rpc(rpc_header: RpcBlockHeaderGenesisToLondon) -> Self {
        let logs_bloom = <Self as BlockHeader>::hex_to_fixed_array::<256>(&rpc_header.logs_bloom);
        let nonce = <Self as BlockHeader>::hex_to_fixed_array::<8>(&rpc_header.nonce);

        BlockHeaderGenesisToLondon {
            parent_hash: H256::from_str(&rpc_header.parent_hash).unwrap(),
            ommers_hash: H256::from_str(&rpc_header.sha3_uncles).unwrap(),
            beneficiary: H160::from_str(&rpc_header.miner).unwrap(),
            state_root: H256::from_str(&rpc_header.state_root).unwrap(),
            transactions_root: H256::from_str(&rpc_header.transactions_root).unwrap(),
            receipts_root: H256::from_str(&rpc_header.receipts_root).unwrap(),
            logs_bloom,
            difficulty: U256::from_str(&rpc_header.difficulty).unwrap(),
            number: U256::from_str(&rpc_header.number).unwrap(),
            gas_limit: U256::from_str(&rpc_header.gas_limit).unwrap(),
            gas_used: U256::from_str(&rpc_header.gas_used).unwrap(),
            timestamp: U256::from_str(&rpc_header.timestamp).unwrap(),
            extra_data: hex::decode(&rpc_header.extra_data[2..]).unwrap_or_default(),
            mix_hash: H256::from_str(&rpc_header.mix_hash).unwrap(),
            nonce,
        }
    }
}

impl BlockHeader for BlockHeaderGenesisToLondon {
    fn rlp_encode(&self) -> Vec<u8> {
        let mut stream = RlpStream::new_list(15);
        stream.append(&self.parent_hash);
        stream.append(&self.ommers_hash);
        stream.append(&self.beneficiary);
        stream.append(&self.state_root);
        stream.append(&self.transactions_root);
        stream.append(&self.receipts_root);
        stream.append(&self.logs_bloom.to_vec());
        stream.append(&self.difficulty);
        stream.append(&self.number);
        stream.append(&self.gas_limit);
        stream.append(&self.gas_used);
        stream.append(&self.timestamp);
        stream.append(&self.extra_data);
        stream.append(&self.mix_hash);
        stream.append(&self.nonce.as_slice());
        stream.out().to_vec()
    }
}

pub fn encode_genesis_to_london(rpc_header: RpcBlockHeaderGenesisToLondon) -> Vec<u8> {
    let header = BlockHeaderGenesisToLondon::from_rpc(rpc_header);

    header.rlp_encode()
}

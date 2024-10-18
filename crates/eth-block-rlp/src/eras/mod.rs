mod genesis_to_london;
mod london_to_paris;
mod paris_to_shanghai;
mod shanghai_to_cancun;

pub use genesis_to_london::{encode_genesis_to_london, RpcBlockHeaderGenesisToLondon};
pub use london_to_paris::{encode_london_to_paris, RpcBlockHeaderLondonToParis};
pub use paris_to_shanghai::{encode_paris_to_shanghai, RpcBlockHeaderParisToShanghai};
pub use shanghai_to_cancun::{encode_shanghai_to_cancun, RpcBlockHeaderShanghaiToCancun};

use alloy::sol;
use alloy_primitives::{B256, keccak256};
use serde::{Deserialize, Serialize};

sol! {
    #[derive(Debug, Deserialize, Serialize)]
    event VampTokenIntent(
        uint256 chainId,
        uint256 blockNumber,
        bytes32 intentId,
        address caller,
        address token,
        string tokenName,
        string tokenSymbol,
        string tokenURI
    );
}

pub fn topic0() -> B256 {
    keccak256("VampTokenIntent(uint256,uint256,bytes32,address,address,string,string,string".as_bytes())
}

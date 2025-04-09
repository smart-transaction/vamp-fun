use sha3::{Digest, Keccak256};

pub fn calculate_hash(data: &[u8]) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

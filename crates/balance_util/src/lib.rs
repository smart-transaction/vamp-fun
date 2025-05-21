use std::error::Error;

use sha3::{Digest, Keccak256};

pub fn get_balance_hash(
    address: &Vec<u8>,
    amount: u64,
    intent_id: &Vec<u8>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut hash_message = Keccak256::new();
    hash_message.update(address);
    hash_message.update(&amount.to_le_bytes());
    hash_message.update(intent_id);
    let hash_message = hash_message.finalize();
    Ok(hash_message.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_balance_hash() {
        // Test setup
        let address = hex::decode("589A698b7b7dA0Bec545177D3963A2741105C7C9").unwrap();
        let amount = 1_000_000_000u64;
        let intent_id = hex::decode("1111111111111111111111111111111111111111111111111111111111111111").unwrap();

        // Test case: Valid inputs
        let balance_hash = get_balance_hash(&address, amount, &intent_id);
        assert!(balance_hash.is_ok());
        assert_eq!(
            balance_hash.unwrap(),
            vec![
                181, 209, 203, 211, 173, 67, 135, 228, 171, 113, 74, 177, 223, 120, 19, 120,
                245, 152, 134, 189, 69, 93, 73, 168, 41, 70, 164, 38, 255, 208, 97, 141
            ]
        );
    }
}
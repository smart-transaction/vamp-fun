use std::error::Error;
use alloy_primitives::U256;
use anyhow::{anyhow, Result};

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

pub fn convert_to_sol(src_amount: &U256) -> Result<(u64, u8)> {
    // Truncate the amount to gwei
    let amount = src_amount
        .checked_div(U256::from(10u64.pow(9)))
        .ok_or(anyhow!("Failed to divide amount"))?;
    // Further truncating until the value fits u64
    // Setting it to zero right now, as we are fixed on decimals = 9.
    // Will be set to 9 later when we can customize decimals On Solana
    let max_extra_decimals = 9u8;
    for decimals in 0..=max_extra_decimals {
        let trunc_amount = amount
            .checked_div(U256::from(10u64.pow(decimals as u32)))
            .ok_or(anyhow!("Failed to divide amount"))?;
        // Check that we are not losing precision
        if trunc_amount
            .checked_mul(U256::from(10u64.pow(decimals as u32)))
            .ok_or(anyhow!("Failed to multiply amount"))?
            != amount
        {
            return Err(anyhow!(
                "The amount {:?} is too large to be minted on Solana",
                amount
            ));
        }
        let max_amount = U256::from(u64::MAX);
        if trunc_amount <= max_amount {
            let val: u64 = trunc_amount
                .try_into()
                .map_err(|_| anyhow!("Failed to convert to u64"))?;
            return Ok((val, 9u8 - decimals));
        }
    }
    Err(anyhow!(
        "The amount {:?} is too large to be minted on Solana",
        amount
    ))
}

pub fn convert_to_sol_with_dec(src_amount: &U256, decimals: u8) -> Result<u64> {
    // Truncate the amount to gwei
    let amount = src_amount
        .checked_div(U256::from(10u64.pow(9)))
        .ok_or(anyhow!("Failed to divide amount"))?;
    let trunc_amount = amount
        .checked_div(U256::from(10u64.pow(decimals as u32)))
        .ok_or(anyhow!("Failed to divide amount"))?;
    // Check that we are not losing precision
    if trunc_amount
        .checked_mul(U256::from(10u64.pow(decimals as u32)))
        .ok_or(anyhow!("Failed to multiply amount"))?
        != amount
    {
        return Err(anyhow!(
            "The amount {:?} is too large to be minted on Solana",
            amount
        ));
    }
    let max_amount = U256::from(u64::MAX);
    if trunc_amount <= max_amount {
        let val: u64 = trunc_amount
            .try_into()
            .map_err(|_| anyhow!("Failed to convert to u64"))?;
        return Ok(val);
    }
    Err(anyhow!(
        "The amount {:?} is too large to be minted on Solana",
        amount
    ))
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

    #[test]
    fn test_convert_to_sol_small_value() {
        let res = convert_to_sol(&U256::from(123456789777000000111u128));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (123456789777, 9));
    }

    #[test]
    fn test_convert_to_sol_large_value() {
        let res = convert_to_sol(&U256::from(123123123456789123000000000000000111u128));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (12312312345678912300, 2));
    }

    #[test]
    fn test_convert_to_sol_too_large_value() {
        let res = convert_to_sol(&U256::from(123123123456789123555555000000000111u128));
        assert!(res.is_err());
    }
}

use anchor_lang::prelude::*;
use crate::state::vamp_state::VampState;
use crate::event::ErrorCode;

// Library function to calculate SOL cost for claiming tokens using bonding curve
pub fn calculate_claim_cost(
    vamp_state: &VampState,
    token_amount: u64,
) -> Result<u64> {
    // Early return if token_amount is 0 to prevent division by zero
    if token_amount == 0 {
        return Ok(0);
    }

    // When not using bonding curve, use fixed flat price per token
    if !vamp_state.use_bonding_curve {
        if !vamp_state.paid_claiming_enabled {
            return Ok(0);
        }
        
        // Safety check: cap flat price to prevent extremely high costs
        let safe_flat_price = std::cmp::min(vamp_state.flat_price_per_token, 1);
        let mut cost = (token_amount as u128)
            .checked_mul(safe_flat_price as u128)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        // Additional safety: cap total cost to prevent extremely high amounts
        let max_total_cost = 100_000_000; // 0.1 SOL maximum
        if cost > max_total_cost {
            cost = max_total_cost;
        }
        
        return Ok(cost.try_into().map_err(|_| ErrorCode::ArithmeticOverflow)?);
    }
    
    let x1 = vamp_state.total_claimed;
    let x2 = x1.checked_add(token_amount).ok_or(ErrorCode::ArithmeticOverflow)?;

    // Part 1: Use a more gradual curve - linear with small slope instead of quadratic
    let delta_tokens = (x2 - x1) as u128;
    let part1 = delta_tokens
        .checked_mul(vamp_state.curve_slope as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_mul(delta_tokens)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(100000) // Divide by 100000 to make the slope much smaller
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    // Part 2: b * (x2 - x1)
    let part2 = delta_tokens
        .checked_mul(vamp_state.base_price as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    let total_cost = part1.checked_add(part2).ok_or(ErrorCode::ArithmeticOverflow)?;

    // Optional: Cap the max cost per token for better UX
    if let Some(max_price_per_token) = vamp_state.max_price {
        let avg_price = total_cost.checked_div(delta_tokens).ok_or(ErrorCode::ArithmeticOverflow)?;
        if avg_price > max_price_per_token as u128 {
            return Err(ErrorCode::PriceTooHigh.into());
        }
    }

    // Final cost in u64
    Ok(total_cost.try_into().map_err(|_| ErrorCode::ArithmeticOverflow)?)
}

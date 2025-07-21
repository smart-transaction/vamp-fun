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
        let cost = (token_amount as u128)
            .checked_mul(vamp_state.flat_price_per_token as u128)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        return Ok(cost.try_into().map_err(|_| ErrorCode::ArithmeticOverflow)?);
    }
    
    let x1 = vamp_state.total_claimed;
    let x2 = x1.checked_add(token_amount).ok_or(ErrorCode::ArithmeticOverflow)?;

    // Part 1: Integral of ax + b over [x1, x2], resulting in a * (x2^2 - x1^2) / 2
    let x1_squared = (x1 as u128).checked_mul(x1 as u128).ok_or(ErrorCode::ArithmeticOverflow)?;
    let x2_squared = (x2 as u128).checked_mul(x2 as u128).ok_or(ErrorCode::ArithmeticOverflow)?;
    let delta_squared = x2_squared.checked_sub(x1_squared).ok_or(ErrorCode::ArithmeticOverflow)?;
    let part1 = delta_squared
        .checked_mul(vamp_state.curve_slope as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(2)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    // Part 2: b * (x2 - x1)
    let delta_tokens = (x2 - x1) as u128;
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

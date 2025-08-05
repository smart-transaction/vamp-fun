use anchor_lang::prelude::*;
use crate::state::vamp_state::VampState;
use crate::event::ErrorCode;
use spl_math::precise_number::PreciseNumber;

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
        
        // Use spl-math for safe multiplication
        let token_amount_precise = PreciseNumber::new(token_amount as u128)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        let flat_price_precise = PreciseNumber::new(safe_flat_price as u128)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        let cost_precise = token_amount_precise
            .checked_mul(&flat_price_precise)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        // Additional safety: cap total cost to prevent extremely high amounts
        let max_total_cost = PreciseNumber::new(100_000_000u128) // 0.1 SOL maximum
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        // Convert to u128 for comparison
        let cost_u128 = cost_precise.floor().ok_or(ErrorCode::ArithmeticOverflow)?.to_imprecise().ok_or(ErrorCode::ArithmeticOverflow)?;
        let max_total_cost_u128 = max_total_cost.floor().ok_or(ErrorCode::ArithmeticOverflow)?.to_imprecise().ok_or(ErrorCode::ArithmeticOverflow)?;
        
        let final_cost_u128 = if cost_u128 > max_total_cost_u128 {
            max_total_cost_u128
        } else {
            cost_u128
        };
        
        return Ok(final_cost_u128.try_into().map_err(|_| ErrorCode::ArithmeticOverflow)?);
    }
    
    let x1 = vamp_state.total_claimed;
    let x2 = x1.checked_add(token_amount).ok_or(ErrorCode::ArithmeticOverflow)?;

    // Convert to PreciseNumber for safe calculations
    let x1_precise = PreciseNumber::new(x1 as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    let x2_precise = PreciseNumber::new(x2 as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    let curve_slope_precise = PreciseNumber::new(vamp_state.curve_slope as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    let base_price_precise = PreciseNumber::new(vamp_state.base_price as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    let divisor = PreciseNumber::new(100000u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    // Calculate delta tokens
    let delta_tokens_precise = x2_precise
        .checked_sub(&x1_precise)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    // Part 1: Use a more gradual curve - linear with small slope instead of quadratic
    let part1 = delta_tokens_precise
        .checked_mul(&curve_slope_precise)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_mul(&delta_tokens_precise)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(&divisor) // Divide by 100000 to make the slope much smaller
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    // Part 2: b * (x2 - x1)
    let part2 = delta_tokens_precise
        .checked_mul(&base_price_precise)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    let total_cost_precise = part1.checked_add(&part2).ok_or(ErrorCode::ArithmeticOverflow)?;

    // Optional: Cap the max cost per token for better UX
    if let Some(max_price_per_token) = vamp_state.max_price {
        let max_price_precise = PreciseNumber::new(max_price_per_token as u128)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        let avg_price_precise = total_cost_precise
            .checked_div(&delta_tokens_precise)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        // Convert to u128 for comparison
        let avg_price_u128 = avg_price_precise.floor().ok_or(ErrorCode::ArithmeticOverflow)?.to_imprecise().ok_or(ErrorCode::ArithmeticOverflow)?;
        let max_price_u128 = max_price_precise.floor().ok_or(ErrorCode::ArithmeticOverflow)?.to_imprecise().ok_or(ErrorCode::ArithmeticOverflow)?;
        
        if avg_price_u128 > max_price_u128 {
            return Err(ErrorCode::PriceTooHigh.into());
        }
    }

    // Convert back to u64 for final result
    let total_cost_u128 = total_cost_precise.floor().ok_or(ErrorCode::ArithmeticOverflow)?.to_imprecise()
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    
    Ok(total_cost_u128.try_into().map_err(|_| ErrorCode::ArithmeticOverflow)?)
}

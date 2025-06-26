use anchor_lang::prelude::*;
use crate::state::vamp_state::VampState;
use crate::event::ErrorCode;

// Library function to calculate SOL cost for claiming tokens using bonding curve
pub fn calculate_claim_cost(
    vamp_state: &VampState,
    token_amount: u64,
) -> Result<u64> {
    // Calculate the SOL cost using the bonding curve formula
    // Since tokens are already minted and in the vault, we calculate based on
    // how many tokens have been claimed (total_claimed) plus the new tokens being claimed
    
    let current_claimed = vamp_state.total_claimed;
    let new_total_claimed = current_claimed.checked_add(token_amount).ok_or(ErrorCode::ArithmeticOverflow)?;
    
    // Calculate the area under the curve from current_claimed to new_total_claimed
    // For quadratic curve: integral = initial_price * (new_total_claimed^2 - current_claimed^2) / 2
    // Use integer arithmetic to avoid overflow
    
    let current_claimed_squared = current_claimed.checked_mul(current_claimed).ok_or(ErrorCode::ArithmeticOverflow)?;
    let new_total_claimed_squared = new_total_claimed.checked_mul(new_total_claimed).ok_or(ErrorCode::ArithmeticOverflow)?;
    
    let area_difference = new_total_claimed_squared.checked_sub(current_claimed_squared).ok_or(ErrorCode::ArithmeticOverflow)?;
    let area_under_curve = area_difference.checked_mul(vamp_state.initial_price).ok_or(ErrorCode::ArithmeticOverflow)?;
    let sol_cost = area_under_curve.checked_div(2).ok_or(ErrorCode::ArithmeticOverflow)?;
    
    Ok(sol_cost)
} 
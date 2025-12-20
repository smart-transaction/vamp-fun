use anchor_lang::prelude::*;
use rust_decimal::prelude::*;

/// Library function to calculate cost for claiming tokens using fixed price
pub fn calculate_claim_cost_fixed_price(
    token_amount: Decimal,
    flat_price_per_token: Decimal,
) -> Result<Decimal> {
    // Early return if token_amount is 0 to prevent division by zero
    if token_amount == Decimal::zero() {
        return Ok(Decimal::zero());
    }
    return Ok(token_amount
        .checked_mul(flat_price_per_token)
        .ok_or_else(|| ProgramError::ArithmeticOverflow)?);
}

/// Library function to calculate cost for claiming tokens using bonding curve
pub fn calculate_claim_cost_bonding_curve(
    token_amount: Decimal,
    total_claimed: Decimal,
    base_price: Decimal,
    curve_slope: Decimal,
) -> Result<Decimal> {
    // Early return if token_amount is 0 to prevent division by zero
    if token_amount == Decimal::zero() {
        return Ok(Decimal::zero());
    }

    let x1 = total_claimed;
    let x2 = x1
        .checked_add(token_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    let divisor = Decimal::new(100000, 0);

    // Calculate delta tokens
    let delta_tokens = x2.checked_sub(x1).ok_or(ProgramError::ArithmeticOverflow)?;

    // Part 1: Use a more gradual curve - linear with small slope instead of quadratic
    let part1 = delta_tokens
        .checked_mul(curve_slope)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_mul(delta_tokens)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(divisor) // Divide by 100000 to make the slope much smaller
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Part 2: b * (x2 - x1)
    let part2 = delta_tokens
        .checked_mul(base_price)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    let total_cost = part1
        .checked_add(part2)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    Ok(total_cost)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_calculate_claim_cost_fixed_price() {
        let claim_cost = calculate_claim_cost_fixed_price(Decimal::new(1, 0), Decimal::new(1, 6));
        assert_eq!(claim_cost, Ok(Decimal::new(1, 6)));
    }

    #[test]
    fn test_calculate_claim_cost_bonding_curve_high_liq() {
        let claim_cost = calculate_claim_cost_bonding_curve(
            Decimal::new(1, 0),
            Decimal::new(100, 0),
            Decimal::new(1, 6),
            Decimal::new(1, 3));
        assert_eq!(claim_cost, Ok(Decimal::new(101, 8)));
    }

    #[test]
    fn test_calculate_claim_cost_bonding_curve_low_liq() {
        let claim_cost = calculate_claim_cost_bonding_curve(
            Decimal::new(1, 0),
            Decimal::new(5, 0),
            Decimal::new(1, 6),
            Decimal::new(1, 3));
        assert_eq!(claim_cost, Ok(Decimal::new(101, 8)));
    }
}

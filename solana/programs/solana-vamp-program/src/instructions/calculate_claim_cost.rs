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

    let base_cost = token_amount
        .checked_mul(base_price)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    let curve_cost = curve_slope
        .checked_mul(
            token_amount
                .checked_mul(total_claimed)
                .ok_or(ProgramError::ArithmeticOverflow)?
                .checked_add(
                    token_amount
                        .checked_mul(
                            token_amount
                                .checked_sub(Decimal::new(1, 0))
                                .ok_or(ProgramError::ArithmeticOverflow)?,
                        )
                        .ok_or(ProgramError::ArithmeticOverflow)?
                        .checked_div(Decimal::new(2, 0))
                        .ok_or(ProgramError::ArithmeticOverflow)?,
                )
                .ok_or(ProgramError::ArithmeticOverflow)?,
        )
        .ok_or(ProgramError::ArithmeticOverflow)?;

    Ok(base_cost.checked_add(curve_cost).ok_or(ProgramError::ArithmeticOverflow)?)
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
    fn test_calculate_claim_cost_bonding_curve_one_three_times() {
        let claim_cost = calculate_claim_cost_bonding_curve(
            Decimal::new(1, 0),
            Decimal::new(0, 0),
            Decimal::new(1, 6),
            Decimal::new(1, 8),
        );
        assert_eq!(claim_cost, Ok(Decimal::new(1, 6)));
        let claim_cost = calculate_claim_cost_bonding_curve(
            Decimal::new(1, 0),
            Decimal::new(1, 0),
            Decimal::new(1, 6),
            Decimal::new(1, 8),
        );
        assert_eq!(claim_cost, Ok(Decimal::new(101, 8)));
        let claim_cost = calculate_claim_cost_bonding_curve(
            Decimal::new(1, 0),
            Decimal::new(2, 0),
            Decimal::new(1, 6),
            Decimal::new(1, 8),
        );
        assert_eq!(claim_cost, Ok(Decimal::new(102, 8)));
    }

    #[test]
    fn test_calculate_claim_cost_bonding_curve_three_one_time() {
        let claim_cost = calculate_claim_cost_bonding_curve(
            Decimal::new(3, 0),
            Decimal::new(0, 0),
            Decimal::new(1, 6),
            Decimal::new(1, 8),
        );
        assert_eq!(claim_cost, Ok(Decimal::new(303, 8)));
    }

    #[test]
    fn test_calculate_claim_cost_bonding_curve_tiny_amount() {
        let claim_cost = calculate_claim_cost_bonding_curve(
            Decimal::new(1, 9),
            Decimal::new(0, 0),
            Decimal::new(1, 6),
            Decimal::new(1, 8),
        );
        assert_eq!(claim_cost, Ok(Decimal::new(303, 8)));
    }
}

# Testing Vamping Parameters

This document describes how to test the new vamping parameters functionality.

## Overview

The vamping parameters allow configuring:
- `paid_claiming_enabled`: Whether users need to pay SOL to claim tokens
- `use_bonding_curve`: Whether to use bonding curve pricing or flat pricing
- `curve_slope`: Controls how quickly the bonding curve price rises
- `base_price`: Minimum price per token in lamports
- `max_price`: Maximum price per token in lamports
- `flat_price_per_token`: Fixed price per token when not using bonding curve

## Parameter Sources

Vamping parameters can be provided in two ways:

1. **Frontend UI (Preferred)**: Parameters are sent via the `additional_data` field in the UserEventProto
2. **Solver Config (Fallback)**: Parameters are set via command-line arguments when starting the solver

If parameters are provided via the frontend UI, they take precedence over the solver config. If not provided, the solver falls back to its configured defaults.

**Note**: The solver config parameters should still be set in deployment scripts as reasonable fallback defaults. This ensures that if the frontend doesn't provide parameters, the system still works with sensible defaults.

## Frontend Parameter Encoding

When sending parameters via the frontend UI, they should be encoded in the `additional_data` field as follows:

```javascript
additionalData: [
  {
    key: keccak256("PaidClaimingEnabled"),
    value: 1.to_little_endian_bytes(),  // bool as uint8 (1 = true, 0 = false)
  },
  {
    key: keccak256("UseBondingCurve"),
    value: 1.to_little_endian_bytes(),  // bool as uint8 (1 = true, 0 = false)
  },
  {
    key: keccak256("CurveSlope"),
    value: 1u64.to_little_endian_bytes(),  // uint64
  },
  {
    key: keccak256("BasePrice"),
    value: 100u64.to_little_endian_bytes(),  // uint64
  },
  {
    key: keccak256("MaxPrice"),
    value: 1000u64.to_little_endian_bytes(),  // uint64
  },
  {
    key: keccak256("FlatPricePerToken"),
    value: 1u64.to_little_endian_bytes(),  // uint64
  }
]
```

## Test Scenarios

### 1. Free Claiming (Default)
```bash
# Run solver with default parameters (free claiming)
cargo run -- \
  --paid-claiming-enabled=false \
  --use-bonding-curve=false \
  --curve-slope=1 \
  --base-price=100 \
  --max-price=1000 \
  --flat-price-per-token=1
```

### 2. Flat Price Claiming
```bash
# Run solver with flat price claiming
cargo run -- \
  --paid-claiming-enabled=true \
  --use-bonding-curve=false \
  --curve-slope=1 \
  --base-price=100 \
  --max-price=1000 \
  --flat-price-per-token=1000  # 0.000001 SOL per token
```

### 3. Bonding Curve Claiming
```bash
# Run solver with bonding curve claiming
cargo run -- \
  --paid-claiming-enabled=true \
  --use-bonding-curve=true \
  --curve-slope=1 \
  --base-price=100 \
  --max-price=1000 \
  --flat-price-per-token=1
```

### 4. Aggressive Bonding Curve
```bash
# Run solver with more aggressive bonding curve
cargo run -- \
  --paid-claiming-enabled=true \
  --use-bonding-curve=true \
  --curve-slope=5 \
  --base-price=50 \
  --max-price=5000 \
  --flat-price-per-token=1
```

## Expected Behavior

### Free Claiming
- Users can claim tokens without paying any SOL
- `calculate_claim_cost` returns 0

### Flat Price Claiming
- Users pay a fixed amount per token
- Cost = `token_amount * flat_price_per_token`
- Capped at 0.1 SOL maximum

### Bonding Curve Claiming
- Price increases as more tokens are claimed
- Formula: `cost = (curve_slope * delta_tokens² / 100000) + (base_price * delta_tokens)`
- Uses spl-math for safe calculations
- Capped by max_price per token

## Testing the Math

The new implementation uses `spl-math` for safe mathematical operations:

1. **Overflow Protection**: All calculations use `PreciseNumber` to prevent overflow
2. **Safe Division**: Division operations are checked for zero divisors
3. **Price Capping**: Maximum prices are enforced for user experience
4. **Gradual Curves**: The bonding curve formula has been improved to prevent irrational prices

## Verification

To verify the changes work:

1. Deploy the updated Solana program
2. Run the solver with different parameter combinations
3. Check that the VampState is initialized with the correct parameters
4. Test claiming with different token amounts
5. Verify that the SOL costs are calculated correctly

## Example Test Cases

### Small Claim (1,000 tokens)
- Flat price: 1,000 * 1000 = 1,000,000 lamports (0.001 SOL)
- Bonding curve: ~110,000 lamports (0.00011 SOL)

### Large Claim (100,000 tokens)
- Flat price: 100,000 * 1000 = 100,000,000 lamports (0.1 SOL) - capped
- Bonding curve: ~110,000,000 lamports (0.11 SOL)

The bonding curve provides more reasonable pricing for large claims while still incentivizing early adoption. 
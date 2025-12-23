# Bonding Curve Implementation

## Overview

This document describes the bonding curve implementation used in the Vamp token system for calculating the cost of claiming tokens.

## Formula

### Current Implementation

The bonding curve uses a linear equation formula:

```
cost = m * P0 ​+ k * (m * S + m(m−1) / 2​)​
```

Where:

- `m` = current total tokens claimed
- `S` = total tokens claimed after purchase
- `k` = controls how quickly price rises
- `P0` = minimum price per token

**Note:** All math is performed using checked arithmetic and `PreciseNumber` for overflow safety.

### Previous Formula (Deprecated)

The original quadratic formula was too aggressive:

```
cost = (k * (S² - m²) / 2) + (P0 * (S - m))
```

## Benefits

1. **Linear Pricing**: The formula provides essentially linear pricing with a small quadratic component
2. **Predictable**: Base price dominates, making costs predictable and manageable
3. **User-Friendly**: Prices scale linearly with token amount for most practical purposes
4. **Safe**: Multiple safety features prevent overflow and extremely high costs

## Pricing Modes

### Free Claiming (Default)

When `paid_claiming_enabled = false`:

- **Cost**: 0 SOL (free claiming)
- **Use case**: Initial token distribution or promotional periods

### Flat Pricing

When `use_bonding_curve = false` and `paid_claiming_enabled = true`:

- **Formula**: `token_amount * flat_price_per_token`
- **Safety cap**: 1 lamport per token maximum
- **Total cap**: 0.1 SOL maximum
- **Use case**: Simple, predictable pricing

### Bonding Curve Pricing

When `use_bonding_curve = true` and `paid_claiming_enabled = true`:

- **Formula**: `(curve_slope * token_amount² / 100000) + (base_price * token_amount)`
- **Use case**: Dynamic pricing that increases with demand

## Safety Features

- **Overflow Protection**: Uses `rust_decimal::Decimal` for all calculations
- **Checked Arithmetic**: All operations use safe math methods

## Implementation Details

- The formula is implemented in `calculate_claim_cost.rs`
- Parameters are configured in `initialize.rs`
- Overflow protection with `Decimal` arithmetic operations
- Support for both bonding curve and flat pricing modes

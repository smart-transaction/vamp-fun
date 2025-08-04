# Bonding Curve Implementation

## Overview

This document describes the bonding curve implementation used in the Vamp token system for calculating the cost of claiming tokens.

## Formula

### Current Implementation

The bonding curve uses a linear equation formula:

```
cost = (curve_slope * (x2 - x1) * (x2 - x1) / 100000) + (base_price * (x2 - x1))
```

Where:

- `x1` = current total tokens claimed
- `x2` = total tokens claimed after purchase
- `curve_slope` = controls how quickly price rises
- `base_price` = minimum price per token
- `100000` = divisor to make the curve more gradual

**Note:** All math is performed using checked arithmetic and `PreciseNumber` for overflow safety.

### Previous Formula (Deprecated)

The original quadratic formula was too aggressive:

```
cost = (curve_slope * (x2² - x1²) / 2) + (base_price * (x2 - x1))
```

## Parameters

| Parameter               | Value                | Description                                                   |
| ----------------------- | -------------------- | ------------------------------------------------------------- |
| `curve_slope`           | 1                    | Controls price growth rate (divided by 100000 in calculation) |
| `base_price`            | 10,000,000 lamports  | Minimum price per token (~0.01 SOL)                           |
| `max_price`             | 100,000,000 lamports | Maximum price per token (~0.1 SOL)                            |
| `flat_price_per_token`  | 1 lamport            | Flat price per token when bonding curve is disabled           |
| `paid_claiming_enabled` | false                | Whether claiming requires payment (default: free)             |
| `use_bonding_curve`     | true                 | Whether to use bonding curve vs flat pricing                  |

## Price Analysis

### Example Calculations

#### 1 Token

- **Part 1**: `1 * (1)² / 100000 = 0.00001` lamports
- **Part 2**: `10,000,000 * 1 = 10,000,000` lamports
- **Total Cost**: `10,000,000` lamports
- **Average Price**: `10,000,000` lamports per token ✅

#### 10 Tokens

- **Part 1**: `1 * (10)² / 100000 = 0.001` lamports
- **Part 2**: `10,000,000 * 10 = 100,000,000` lamports
- **Total Cost**: `100,000,000` lamports
- **Average Price**: `10,000,000` lamports per token ✅

#### 100 Tokens

- **Part 1**: `1 * (100)² / 100000 = 0.1` lamports
- **Part 2**: `10,000,000 * 100 = 1,000,000,000` lamports
- **Total Cost**: `1,000,000,000` lamports
- **Average Price**: `10,000,000` lamports per token ✅

#### 1,000 Tokens

- **Part 1**: `1 * (1,000)² / 100000 = 10,000` lamports
- **Part 2**: `10,000,000 * 1,000 = 10,000,000,000` lamports
- **Total Cost**: `10,010,000,000` lamports
- **Average Price**: `10,010,000` lamports per token ✅

#### 10,000 Tokens

- **Part 1**: `1 * (10,000)² / 100000 = 1,000,000` lamports
- **Part 2**: `10,000,000 * 10,000 = 100,000,000,000` lamports
- **Total Cost**: `101,000,000,000` lamports
- **Average Price**: `10,100,000` lamports per token ✅

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

- **Overflow Protection**: Uses `spl_math::precise_number::PreciseNumber` for all calculations
- **Early Returns**: Handles edge cases (zero tokens, disabled payment)
- **Price Caps**: Prevents extremely high costs
- **Checked Arithmetic**: All operations use safe math methods

## Implementation Details

- The formula is implemented in `calculate_claim_cost.rs`
- Parameters are configured in `initialize.rs`
- Early return for zero token amounts to prevent division by zero
- Overflow protection with `PreciseNumber` arithmetic operations
- Maximum price cap for better user experience
- Support for both bonding curve and flat pricing modes

# Bonding Curve Implementation

## Overview

This document describes the bonding curve implementation used in the Vamp token system for calculating the cost of claiming tokens.

## Formula

### Current Implementation

The bonding curve uses a modified formula to provide a more gradual price increase:

```
cost = (curve_slope * (x2 - x1)² / 1000) + (base_price * (x2 - x1))
```

Where:

- `x1` = current total tokens claimed
- `x2` = total tokens claimed after purchase
- `curve_slope` = controls how quickly price rises
- `base_price` = minimum price per token
- `1000` = divisor to make the curve more gradual

### Previous Formula (Deprecated)

The original quadratic formula was too aggressive:

```
cost = (curve_slope * (x2² - x1²) / 2) + (base_price * (x2 - x1))
```

## Parameters

| Parameter     | Value          | Description                                                 |
| ------------- | -------------- | ----------------------------------------------------------- |
| `curve_slope` | 10             | Controls price growth rate (divided by 1000 in calculation) |
| `base_price`  | 1,000 lamports | Minimum price per token (~0.000001 SOL)                     |
| `max_price`   | 10,000 lamports | Maximum price per token (~0.00001 SOL)                     |

## Price Analysis

### Example Calculations

#### 1,000 Tokens

- **Part 1**: `10 * (1,000)² / 1000 = 10,000` lamports
- **Part 2**: `1,000 * 1,000 = 1,000,000` lamports
- **Total Cost**: `1,010,000` lamports
- **Average Price**: `1,010` lamports per token ✅

#### 10,000 Tokens

- **Part 1**: `10 * (10,000)² / 1000 = 1,000,000` lamports
- **Part 2**: `1,000 * 10,000 = 10,000,000` lamports
- **Total Cost**: `11,000,000` lamports
- **Average Price**: `1,100` lamports per token ✅

#### 100,000 Tokens

- **Part 1**: `10 * (100,000)² / 1000 = 100,000,000` lamports
- **Part 2**: `1,000 * 100,000 = 100,000,000` lamports
- **Total Cost**: `200,000,000` lamports
- **Average Price**: `2,000` lamports per token ✅

## Benefits

1. **Gradual Price Increase**: The new formula creates a much more manageable price growth
2. **User-Friendly**: Prices stay within reasonable limits even for large purchases
3. **Predictable**: Linear growth with square of token amount (divided by 1000)
4. **Scalable**: Works well for both small and large token amounts

## Implementation Details

- The formula is implemented in `calculate_claim_cost.rs`
- Parameters are configured in `initialize.rs`
- Early return for zero token amounts to prevent division by zero
- Overflow protection with checked arithmetic operations
- Maximum price cap for better user experience

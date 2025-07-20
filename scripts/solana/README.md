# Solana Vamp State Debugging Tools

This directory contains tools to help debug `InvalidValidatorSignature` errors in the vamp.fun Solana program.

## Tools

### 1. Node.js Inspector (`inspect_vamp_state.js`)

A comprehensive Node.js script that can:
- Fetch all VampState accounts from the program
- Parse and display all account data
- Find the specific VampState for your mint address
- Save debug information to a JSON file

**Setup:**
```bash
cd scripts/solana
npm install
```

**Usage:**
```bash
npm run inspect
# or
node inspect_vamp_state.js
```

### 2. Shell Script Inspector (`inspect_vamp_state.sh`)

A simple shell script that provides:
- Basic account inspection using Solana CLI
- Debugging tips and account structure information
- Manual inspection commands

**Usage:**
```bash
./inspect_vamp_state.sh
```

## Debugging InvalidValidatorSignature

The `InvalidValidatorSignature` error occurs when the validator signature verification fails in the claim instruction. Here's what to check:

### 1. Verify Validator Public Key

The validator public key stored in the VampState should match the validator's Ethereum address. Use the inspection tools to verify:

```bash
cd scripts/solana
npm run inspect
```

Look for the `Validator Public Key` field in the output.

### 2. Check Message Format

The validator signature should be created from this message:
```
keccak256(eth_address || balance || intent_id)
```

Where:
- `eth_address`: 20-byte Ethereum address
- `balance`: 8-byte balance amount (little-endian)
- `intent_id`: The intent ID bytes

### 3. Verify Signature Creation

The validator should sign the message using their private key. The signature verification in the Solana program expects:

1. Ethereum signature format (65 bytes: r, s, v)
2. Proper message recovery using secp256k1
3. Recovered address should match the stored validator public key

### 4. Common Issues

- **Wrong validator address**: The validator public key in VampState doesn't match the actual validator
- **Incorrect message format**: The message being signed doesn't match what the program expects
- **Signature format**: The signature isn't in the correct Ethereum format
- **Endianness**: Balance should be in little-endian format

## Account Structure

The VampState account has this structure:
```
- bump: u8 (1 byte)
- mint: Pubkey (32 bytes)
- solver_public_key: Vec<u8> (4 bytes length + up to 20 bytes)
- validator_public_key: Vec<u8> (4 bytes length + up to 20 bytes)
- vamp_identifier: u64 (8 bytes)
- intent_id: Vec<u8> (4 bytes length + up to 32 bytes)
- total_claimed: u64 (8 bytes)
- reserve_balance: u64 (8 bytes)
- token_supply: u64 (8 bytes)
- curve_exponent: u64 (8 bytes)
- initial_price: u64 (8 bytes)
- sol_vault: Pubkey (32 bytes)
```

## Example Output

When you run the Node.js inspector, you'll see output like:
```
🎯 FOUND TARGET VAMP STATE!
=====================================
Validator Public Key: 0x8b25ed06e216553f8d4265996061b0a065afa35c
Solver Public Key: 0xf98b828b389bef4ebbb5911ca17e4f7989c9068d
Intent ID: 0x1111111111111111222222222222222277777777777777779999999999999999
=====================================
```

This will help you verify that the validator public key matches what you expect. 
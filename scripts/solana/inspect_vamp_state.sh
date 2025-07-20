#!/bin/bash

# Vamp State Inspector Script
# This script helps debug InvalidValidatorSignature errors by inspecting vamp state accounts

set -e

# Configuration
PROGRAM_ID="CABA3ibLCuTDcTF4DQXuHK54LscXM5vBg7nWx1rzPaJH"
MINT_ADDRESS="FNYH2GXJztxyVxtTXzJ4co9qL6VeT6VeT6Kry8KgWHQwYfHB"
CLUSTER="devnet"

echo "🔍 Vamp State Inspector"
echo "======================"
echo "Program ID: $PROGRAM_ID"
echo "Mint Address: $MINT_ADDRESS"
echo "Cluster: $CLUSTER"
echo ""

# Function to convert hex to bytes
hex_to_bytes() {
    echo "$1" | sed 's/0x//' | xxd -r -p
}

# Function to convert bytes to hex
bytes_to_hex() {
    echo "$1" | xxd -p | tr -d '\n'
}

echo "📡 Fetching all VampState accounts..."
echo ""

# Get all accounts owned by the program
ACCOUNTS=$(solana account --output json $PROGRAM_ID 2>/dev/null || echo "[]")

if [ "$ACCOUNTS" = "[]" ]; then
    echo "❌ No accounts found for program $PROGRAM_ID"
    echo ""
    echo "Trying alternative approach..."
    echo ""
    
    # Try to get program accounts using a different method
    echo "🔍 Searching for VampState accounts by discriminator..."
    
    # VampState discriminator: [222, 91, 2, 48, 244, 96, 192, 196]
    DISCRIMINATOR="de5b0230f460c0c4"
    
    # This is a simplified approach - in practice you'd need to use the Solana CLI
    # or a more sophisticated tool to filter by account discriminator
    
    echo "⚠️  Note: Direct discriminator filtering requires programmatic access."
    echo "   Consider using the Node.js script (inspect_vamp_state.js) for better results."
    echo ""
    
    # Try to find accounts by looking at recent transactions
    echo "🔍 Checking recent transactions for the mint address..."
    RECENT_TXS=$(solana transaction-history --output json $MINT_ADDRESS 2>/dev/null || echo "[]")
    
    if [ "$RECENT_TXS" != "[]" ]; then
        echo "Found recent transactions for mint address"
        echo "Look for transactions involving the vamp program to find the VampState account"
    else
        echo "No recent transactions found for mint address"
    fi
    
else
    echo "Found accounts for program"
    echo "$ACCOUNTS" | jq -r '.'
fi

echo ""
echo "💡 Debugging Tips:"
echo "=================="
echo "1. The validator_public_key in VampState should match the validator's Ethereum address"
echo "2. Check that the validator signature was created with the correct private key"
echo "3. Verify the message format: keccak256(eth_address || balance || intent_id)"
echo "4. Ensure the validator address is in the correct format (20 bytes)"
echo ""
echo "🔧 To manually inspect an account:"
echo "   solana account <VAMP_STATE_ADDRESS> --output json"
echo ""
echo "📝 Expected VampState structure:"
echo "   - bump: u8"
echo "   - mint: Pubkey (32 bytes)"
echo "   - solver_public_key: Vec<u8> (max 20 bytes)"
echo "   - validator_public_key: Vec<u8> (max 20 bytes)"
echo "   - vamp_identifier: u64"
echo "   - intent_id: Vec<u8> (max 32 bytes)"
echo "   - total_claimed: u64"
echo "   - reserve_balance: u64"
echo "   - token_supply: u64"
echo "   - curve_exponent: u64"
echo "   - initial_price: u64"
echo "   - sol_vault: Pubkey (32 bytes)"
echo ""

# If you have a specific VampState address, you can inspect it directly
if [ ! -z "$VAMP_STATE_ADDRESS" ]; then
    echo "🔍 Inspecting specific VampState account: $VAMP_STATE_ADDRESS"
    echo ""
    solana account $VAMP_STATE_ADDRESS --output json | jq -r '.'
fi

echo ""
echo "✅ Script completed. Check the output above for debugging information." 
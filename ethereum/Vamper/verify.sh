#!/bin/bash
# Usage: ./verify.sh <network-type> <contract-name> <contract-address> [chains] [constructor-sig] [constructor-args]

# Examples:
#   # Verify on all TESTNET chains without constructor arguments
#   ./verify.sh testnet MyContract 0x123456789abcdef
#
#   # Verify only on sepolia (TESTNET)
#   ./verify.sh testnet MyContract 0x123456789abcdef sepolia
#
#   # Verify on multiple TESTNET chains
#   ./verify.sh testnet MyContract 0x123456789abcdef sepolia,polygon_amoy
#
#   # Verify with constructor arguments on all chains
#   ./verify.sh testnet MyContract 0x123456789abcdef "constructor(uint256,address)" "123 0xabc"
#
#   # Verify on specific chain with constructor arguments
#   ./verify.sh testnet MyContract 0x123456789abcdef sepolia "constructor(bool)" "true"
#

set -eo pipefail

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration paths
CONFIG_FILE="config/networks.json"

# Error handling function
error_exit() {
    echo -e "${RED}❌ Error: $1${NC}" >&2
    echo -e "${CYAN}Usage: ./verify.sh <mainnet|testnet> <contract-name> <contract-address> [chains] [constructor-sig] [constructor-args]${NC}" >&2
    exit 1
}

# Validate arguments
if [[ -z "$1" || -z "$2" || -z "$3" ]]; then
    error_exit "Missing required arguments"
fi

# Parse main parameters
NETWORK_TYPE=$(echo "$1" | tr '[:lower:]' '[:upper:]')
CONTRACT_NAME=$2
CONTRACT_ADDRESS=$3
shift 3

# Initialize variables
CHAINS=()
CONSTRUCTOR_SIG=""
CONSTRUCTOR_ARGS=""

# Handle optional chains parameter for mainnet/testnet
if [[ "$NETWORK_TYPE" == "MAINNET" || "$NETWORK_TYPE" == "TESTNET" ]]; then
    if [[ -n "$1" && ! "$1" =~ ^constructor* ]]; then
        IFS=',' read -ra CHAINS <<< "$1"
        shift
    fi
fi

# Handle constructor arguments
if [ -n "$1" ]; then
    CONSTRUCTOR_SIG=$1
    shift
    if [ -n "$1" ]; then
        CONSTRUCTOR_ARGS=$1
        shift
    fi
fi

# Validate network type
valid_networks=("MAINNET" "TESTNET" "LESTNET")
if [[ ! " ${valid_networks[@]} " =~ " ${NETWORK_TYPE} " ]]; then
    error_exit "Invalid network type: '$NETWORK_TYPE'"
fi

echo -e "${YELLOW}⚡ Starting verification process...${NC}"
echo -e "Network type: ${GREEN}$NETWORK_TYPE${NC}"
echo -e "Contract name: ${GREEN}$CONTRACT_NAME${NC}"
echo -e "Contract address: ${GREEN}$CONTRACT_ADDRESS${NC}"

# Encode constructor args if provided
if [ -n "$CONSTRUCTOR_SIG" ]; then
    echo -e "${GREEN}✓ Using constructor arguments${NC}"
    ENCODED_ARGS=$(cast abi-encode "$CONSTRUCTOR_SIG" $CONSTRUCTOR_ARGS)
else
    echo -e "${YELLOW}✓ No constructor signature provided${NC}"
fi

# Load environment variables
if [ -f .env ]; then
    echo -e "${YELLOW}🔧 Loading environment variables...${NC}"
    set -a
    source .env
    set +a
else
    echo -e "${YELLOW}⚠️  No .env file found${NC}"
fi

# Read network configuration
echo -e "${YELLOW}📖 Loading network configuration...${NC}"
NETWORK_CONFIG=$(jq ".${NETWORK_TYPE}" "$CONFIG_FILE")

if [ "$NETWORK_CONFIG" == "null" ]; then
    error_exit "Network configuration not found for $NETWORK_TYPE"
fi

# Verification function
verify_contract() {
    local rpc_url=$1
    local chain_id=$2
    local verifier=$3
    local verifier_url=$4
    local api_key=$5

    local contract_path="src/${CONTRACT_NAME}.sol:${CONTRACT_NAME}"

    echo -e "${CYAN}🔍 Verifying on chain $chain_id ($verifier)...${NC}"
    
    local base_command="forge verify-contract \
        --rpc-url \"$rpc_url\" \
        $CONTRACT_ADDRESS \
        \"$contract_path\" \
        --verifier $verifier \
        --verifier-url \"$verifier_url\""

    if [ -n "$CONSTRUCTOR_SIG" ]; then
        base_command+=" --constructor-args $ENCODED_ARGS"
    else
        base_command+=" --constructor-args \"0x\""
    fi

    if [ "$verifier" == "etherscan" ]; then
        base_command+=" --etherscan-api-key \"$api_key\""
    fi
    echo "base_command: $base_command"

    eval "$base_command"
}

# Process verification based on network type
if [ "$NETWORK_TYPE" == "MAINNET" ] || [ "$NETWORK_TYPE" == "TESTNET" ]; then
    # Get valid chain names
    VALID_CHAINS=()
    length=$(echo "$NETWORK_CONFIG" | jq '. | length')
    for ((i=0; i<length; i++)); do
        VALID_CHAINS+=("$(echo "$NETWORK_CONFIG" | jq -r ".[$i].name")")
    done

    # Validate requested chains
    FILTERED_CHAINS=()
    for chain in "${CHAINS[@]}"; do
        if [[ " ${VALID_CHAINS[@]} " =~ " ${chain} " ]]; then
            FILTERED_CHAINS+=("$chain")
        else
            echo -e "${RED}⚠️ Chain '$chain' not found in $NETWORK_TYPE config${NC}"
        fi
    done

    # If chains specified but none valid, exit
    if [ ${#CHAINS[@]} -gt 0 ] && [ ${#FILTERED_CHAINS[@]} -eq 0 ]; then
        error_exit "No valid chains specified for verification"
    fi

    # Process chains
    for ((i=0; i<length; i++)); do
        chain_config=$(echo "$NETWORK_CONFIG" | jq ".[$i]")
        chain_name=$(echo "$chain_config" | jq -r '.name')

        # Skip if not in filter
        if [ ${#FILTERED_CHAINS[@]} -gt 0 ] && 
           [[ ! " ${FILTERED_CHAINS[@]} " =~ " ${chain_name} " ]]; then
            echo -e "${YELLOW}⏩ Skipping $chain_name${NC}"
            continue
        fi

        # Get chain config
        rpc_env_key=$(echo "$chain_config" | jq -r '.rpcEnvKey')
        chain_id=$(echo "$chain_config" | jq -r '.chainId')
        verifier=$(echo "$chain_config" | jq -r '.verifier')
        verifier_url=$(echo "$chain_config" | jq -r '.verifierUrl')
        api_key_env=$(echo "$chain_config" | jq -r '.apiKeyEnv')

        # Get values from environment
        rpc_url=${!rpc_env_key}
        api_key=${!api_key_env}

        if [ -z "$rpc_url" ] || [ -z "$api_key" ]; then
            echo -e "${RED}⚠️  Missing environment variables for $chain_name${NC}"
            continue
        fi

        verify_contract "$rpc_url" "$chain_id" "$verifier" "$verifier_url" "$api_key"
        sleep 30
    done
else
    # Process LESTNET single configuration
    chain_config=$(echo "$NETWORK_CONFIG" | jq '.')
    rpc_env_key=$(echo "$chain_config" | jq -r '.rpcEnvKey')
    verifier=$(echo "$chain_config" | jq -r '.verifier')
    verifier_url=$(echo "$chain_config" | jq -r '.verifierUrl')
    api_key_env=$(echo "$chain_config" | jq -r '.apiKeyEnv')

    rpc_url=${!rpc_env_key}
    api_key=${!api_key_env}

    if [ -z "$rpc_url" ] || [ -z "$api_key" ]; then
        error_exit "Missing LESTNET environment variables"
    fi

    verify_contract "$rpc_url" "$chain_id" "$verifier" "$verifier_url" "$api_key"
fi

echo -e "${GREEN}✅ Successfully verified $CONTRACT_NAME at $CONTRACT_ADDRESS on $NETWORK_TYPE${NC}"


#!/bin/bash

# Usage: ./deploy.sh <network-type> <contract-name> [chains] [count] [names-array] [symbols-array]
# Example: ./deploy.sh testnet MockERC20 '["chain"]' 2 '["Token1","Token2"]' '["T1","T2"]'

set -eo pipefail

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

valid_networks=("MAINNET" "TESTNET")

# Error handling
error_exit() {
    echo -e "${RED}❌ Error: $1${NC}" >&2
    exit 1
}

# Check for jq dependency
if ! command -v jq &> /dev/null; then
    error_exit "jq is required. Install with: sudo apt-get install jq"
fi

# Validate minimum arguments
if [[ -z "$1" || -z "$2" ]]; then
    error_exit "Missing arguments\nUsage: ./deploy.sh <network-type> <contract-name> [chains] [count] [names-array] [symbols-array]"
fi

NETWORK_TYPE=$1
CONTRACT_NAME=$2
shift 2

# Initialize parameters
TARGET_CHAINS=""
DEPLOY_COUNT=1
declare -a NAMES=()
declare -a SYMBOLS=()

# Process target chains (optional)
if [[ "$1" =~ ^\[.*\]$ ]]; then
    TARGET_CHAINS="$1"
    shift
fi

# Salt
SALT=$(date +%s%N)
# Fee in WEI
# 10 GWei
FEE=10000000000

# Convert network type to uppercase
NETWORK_TYPE=$(echo "$NETWORK_TYPE" | tr '[:lower:]' '[:upper:]')
[[ ! " ${valid_networks[@]} " =~ " ${NETWORK_TYPE} " ]] && error_exit "Invalid NETWORK_TYPE: '$NETWORK_TYPE'"

# Environment setup
[ -f .env ] && source .env
export NETWORK_TYPE=$NETWORK_TYPE
[[ -n "$TARGET_CHAINS" ]] && export TARGET_CHAINS="$TARGET_CHAINS"

# Deployment messages
echo -e "${YELLOW}⚡ Starting deployment...${NC}"
echo -e "• Network: ${GREEN}$NETWORK_TYPE${NC}"
echo -e "• Contract: ${GREEN}$CONTRACT_NAME${NC}"
[[ "$CONTRACT_NAME" == "MockERC20" ]] && echo -e "• Instances: ${GREEN}$DEPLOY_COUNT${NC}"
[[ -n "$SALT" ]] && echo -e "• Salt: ${GREEN}$SALT${NC}"
echo -e "• Chains: ${GREEN}${TARGET_CHAINS:-all networks}${NC}"

# Verify deployment script exists
SCRIPT_PATH="script/Deploy${CONTRACT_NAME}.s.sol"
[ ! -f "$SCRIPT_PATH" ] && error_exit "Deployment script not found: $SCRIPT_PATH"

FORGE_CMD="forge script $SCRIPT_PATH --broadcast -vvvvv --ffi"
FORGE_CMD+=" --sig \"run(uint256,uint256)\" $SALT $FEE"

echo ${FORGE_CMD}
eval "$FORGE_CMD"

echo -e "\n${GREEN}✅ Successfully deployed ${DEPLOY_COUNT} ${CONTRACT_NAME} instances!${NC}"
echo -e "${YELLOW}⏱  Completed in ${SECONDS} seconds${NC}"

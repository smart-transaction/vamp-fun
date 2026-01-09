. .env

export PORT=9010
export SOLANA_DEVNET_URL="https://api.devnet.solana.com"
export SOLANA_MAINNET_URL="https://api.mainnet-beta.solana.com"
export MYSQL_HOST="localhost"
export MYSQL_PORT=3306
export MYSQL_USER="vamper"
export MYSQL_PASSWORD="secret2"
export MYSQL_DATABASE="vampfun"
export POLL_FREQUENCY_SECS=5
export ETHEREUM_PRIVATE_KEY="${ETHEREUM_PRIVATE_KEY}"
export SOLANA_PRIVATE_KEY="${SOLANA_PRIVATE_KEY}"
export DEFAULT_SOLANA_CLUSTER="DEVNET"
export PAID_CLAIMING_ENABLED=true
export USE_BONDING_CURVE=true
export CURVE_SLOPE=1000  # Decimals = 9, val=1e-6
export BASE_PRICE=10000000  # Decimals = 9, val=1e-2 tokens
export MAX_PRICE=1000000000  # Decimals = 9, val=1 token
export FLAT_PRICE_PER_TOKEN=10000000  # Decimals = 9, val=1e-2 tokens
export AMQP_HOST="localhost"
export AMQP_PORT="5672"
export AMQP_USER="guest"
export AMQP_PASSWORD="guest"
export EXCHANGE_NAME="vamp-fun"
export ROUTING_KEY="event.vamp"
export QUEUE_NAME="solver-queue"

cargo run

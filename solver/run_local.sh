. .env

export PORT=9010
export SOLANA_DEVNET_URL=https://red-burned-rain.solana-devnet.quiknode.pro/${QUICKNODE_API_KEY}
export SOLANA_MAINNET_URL=
export MYSQL_USER=server
export MYSQL_PASSWORD=secret2
export MYSQL_HOST=localhost
export MYSQL_PORT=3306
export MYSQL_DATABASE=vampfun
export REQUEST_REGISTRATOR_URL=http://localhost:50051
export ORCHESTRATOR_URL=http://localhost:50052
export VALIDATOR_URL=http://localhost:50053
export POLL_FREQUENCY_SECS=5
export ETHEREUM_PRIVATE_KEY=${ETHEREUM_PRIVATE_KEY}
export SOLANA_PRIVATE_KEY=${SOLANA_PRIVATE_KEY}
export DEFAULT_SOLANA_CLUSTER=DEVNET
export PAID_CLAIMING_ENABLED=true
export USE_BONDING_CURVE=true
export CURVE_SLOPE=1000  # Decimals = 9, val=1e-6
export BASE_PRICE=10000000  # Decimals = 9, val=1e-2 tokens
export MAX_PRICE=1000000000  # Decimals = 9, val=1 token
export FLAT_PRICE_PER_TOKEN=10000000  # Decimals = 9, val=1e-2 tokens
export AMQP_HOST=
export AMQP_PORT=
export AMQP_USER=
export AMQP_PASSWORD=
export EXCHANGE_NAME=
export QUEUE_NAME=
export ROUTING_KEY=

cargo run

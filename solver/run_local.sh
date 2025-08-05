MYSQL_USER=server
MYSQL_PASSWORD=secret2
MYSQL_HOST=localhost
MYSQL_PORT=3306
MYSQL_DATABASE=vampfun
REQUEST_REGISTRATOR_URL=http://localhost:50051
ORCHESTRATOR_URL=http://localhost:50052
VALIDATOR_URL=http://localhost:50053
POLL_FREQUENCY_SECS=5
PRIVATE_KEY=0xa16244600268d2379a6e22b0dc1d6064d714b43b346a434f3fd50831103f56bf
SOLANA_PRIVATE_KEY=5uw3Qy49XG31tVAScho8Ww32A9bjXc3iDVtzdvtPRK6iz3hmp9XYFKBcGNJ1j53gUXoiQDQcFQDuhcmb4ieMb4bR
DEFAULT_SOLANA_CLUSTER=DEVNET

cargo run \
  -- \
  --request-registrator-url=${REQUEST_REGISTRATOR_URL} \
  --orchestrator-url=${ORCHESTRATOR_URL} \
  --validator-url=${VALIDATOR_URL} \
  --mysql-user=${MYSQL_USER} \
  --mysql-password=${MYSQL_PASSWORD} \
  --mysql-host=${MYSQL_HOST} \
  --mysql-port=${MYSQL_PORT} \
  --mysql-database=${MYSQL_DATABASE} \
  --poll-frequency-secs=${POLL_FREQUENCY_SECS} \
  --private-key=${PRIVATE_KEY} \
  --solana-private-key=${SOLANA_PRIVATE_KEY} \
  --default-solana-cluster=${DEFAULT_SOLANA_CLUSTER}
  
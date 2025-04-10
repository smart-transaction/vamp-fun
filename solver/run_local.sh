MYSQL_USER=server
MYSQL_PASSWORD=secret2
MYSQL_HOST=localhost
MYSQL_PORT=3306
MYSQL_DATABASE=vampfun
REQUEST_REGISTRATOR_URL=http://localhost:50051
ORCHESTRATOR_URL=http://localhost:50052
POLL_FREQUENCY_SECS=5

cargo run \
  -- \
  --request-registrator-url=${REQUEST_REGISTRATOR_URL} \
  --orchestrator-url=${ORCHESTRATOR_URL} \
  --mysql-user=${MYSQL_USER} \
  --mysql-password=${MYSQL_PASSWORD} \
  --mysql-host=${MYSQL_HOST} \
  --mysql-port=${MYSQL_PORT} \
  --mysql-database=${MYSQL_DATABASE} \
  --poll-frequency-secs=${POLL_FREQUENCY_SECS}
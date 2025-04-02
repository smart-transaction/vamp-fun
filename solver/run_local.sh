MYSQL_USER=server
MYSQL_PASSWORD=secret2
MYSQL_HOST=localhost
MYSQL_PORT=3306
MYSQL_DATABASE=vampfun

cargo run \
  -- \
  --mysql-user=${MYSQL_USER} \
  --mysql-password=${MYSQL_PASSWORD} \
  --mysql-host=${MYSQL_HOST} \
  --mysql-port=${MYSQL_PORT} \
  --mysql-database=${MYSQL_DATABASE}
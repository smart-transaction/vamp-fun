# Full stxn solver setup on a clean Linux machine.
#
# Pre-reqs:
# 1. Linux machine: Debian/Ubuntu/...
# 2. setup.sh file from our setup folder locally in a local folder
#    (pulled from Github or otherwise).

set -e

# Choose the environment
PS3="Please choose the environment: "
options=("dev" "prod" "quit")
select OPT in "${options[@]}"
do
  case ${OPT} in
    "dev")
        echo "Using dev environment"
        MYSQL_PASSWORD_VERSION=1
        MYSQL_USER="server"
        MYSQL_HOST="vamp_fun_db"
        MYSQL_PORT=3306
        MYSQL_DATABASE="vampfun"
        PORT=8000
        REQUEST_REGISTRATOR_URL="http://vamp_fun_request_registrator:50051"
        ORCHESTRATOR_URL="http://vamp_fun_orchestrator:50052"
        POLL_FREQUENCY_SECS=5
        REQUEST_REGISTRATOR_ETHEREUM_RPC_URL="wss://service.lestnet.org:8888"
        REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS="0x81D0da49057BCC8b2f5c57bfb43A298C2f634362"
        REQUEST_REGISTRATOR_GRPC_ADDRESS="[::]:50051"
        REQUEST_REGISTRATOR_STORAGE_REDIS_URL="redis://vamp_fun_redis:6379"
        ORCHESTRATOR_SOLANA_CLUSTER="Devnet"
        ORCHESTRATOR_SOLANA_PROGRAM_ADDRESS="5zKTcVqXKk1vYGZpK47BvMo8fwtUrofroCdzSK931wVc"
        ORCHESTRATOR_GRPC_ADDRESS="[::]:50052"
        ORCHESTRATOR_STORAGE_REDIS_URL="redis://vamp_fun_redis:6379"
        break
        ;;
    "prod")
        echo "Using prod environment"
        MYSQL_PASSWORD_VERSION=1
        MYSQL_USER="server"
        MYSQL_HOST="vamp_fun_db"
        MYSQL_PORT=3306
        MYSQL_DATABASE="vampfun"
        PORT=8000
        REQUEST_REGISTRATOR_URL="http://vamp_fun_request_registrator:50051"
        ORCHESTRATOR_URL="http://vamp_fun_orchestrator:50052"
        POLL_FREQUENCY_SECS=5
        REQUEST_REGISTRATOR_ETHEREUM_RPC_URL="wss://service.lestnet.org:8888"
        REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS="0x81D0da49057BCC8b2f5c57bfb43A298C2f634362"
        REQUEST_REGISTRATOR_GRPC_ADDRESS="[::]:50051"
        REQUEST_REGISTRATOR_STORAGE_REDIS_URL="redis://vamp_fun_redis:6379"
        ORCHESTRATOR_SOLANA_CLUSTER="Devnet"
        ORCHESTRATOR_SOLANA_PROGRAM_ADDRESS="5zKTcVqXKk1vYGZpK47BvMo8fwtUrofroCdzSK931wVc"
        ORCHESTRATOR_GRPC_ADDRESS="[::]:50052"
        ORCHESTRATOR_STORAGE_REDIS_URL="redis://vamp_fun_redis:6379"
        break
        ;;
    "quit")
        exit
        ;;
    *) echo "invalid option $REPLY";;
  esac
done

SECRET_SUFFIX=$(echo ${OPT} | tr '[a-z]' '[A-Z]')

# Create necessary files.
cat >up.sh << UP
# Turn up solver.
set -e

# Secrets
cat >.env << ENV
MYSQL_ROOT_PASSWORD=\$(gcloud secrets versions access ${MYSQL_PASSWORD_VERSION} --secret="VAMP_FUN_MYSQL_ROOT_PASSWORD_${SECRET_SUFFIX}")
MYSQL_APP_PASSWORD=\$(gcloud secrets versions access ${MYSQL_PASSWORD_VERSION} --secret="VAMP_FUN_MYSQL_APP_PASSWORD_${SECRET_SUFFIX}")
MYSQL_READER_PASSWORD=\$(gcloud secrets versions access ${MYSQL_PASSWORD_VERSION} --secret="VAMP_FUN_MYSQL_READER_PASSWORD_${SECRET_SUFFIX}")
SOLANA_PRIVATE_KEY=\$(gcloud secrets versions access 1 --secret="VAMP_FUN_SOLANA_PRIVATE_KEY_${SECRET_SUFFIX}")

ENV

sudo docker compose up -d --remove-orphans

rm -f .env

UP

sudo chmod a+x up.sh

cat >down.sh << DOWN
# Turn down solver.
sudo docker compose down
DOWN
sudo chmod a+x down.sh

# Docker images
DOCKER_LOCATION="us-central1-docker.pkg.dev"
DOCKER_PREFIX="${DOCKER_LOCATION}/solver-438012/solver-docker-repo"
SOLVER_DOCKER_IMAGE="${DOCKER_PREFIX}/vampfun-solver-image:${OPT}"
DB_DOCKER_IMAGE="${DOCKER_PREFIX}/vampfun-db-image:live"
ORCHESTRATOR_DOCKER_IMAGE="${DOCKER_PREFIX}/vampfun-orchestrator-image:${OPT}"
REQUEST_REGISTRATOR_DOCKER_IMAGE="${DOCKER_PREFIX}/vampfun-request-registrator-image:${OPT}"
REDIS_DOCKER_IMAGE=redis/redis-stack-server:latest

# Create docker-compose.yml file.
cat >docker-compose.yml << COMPOSE

services:
  vamp_fun_solver:
    container_name: vamp_fun_solver
    image: ${SOLVER_DOCKER_IMAGE}
    restart: unless-stopped
    depends_on:
      vamp_fun_db:
        condition: service_started
      vamp_fun_request_registrator:
        condition: service_started
      vamp_fun_orchestrator:
        condition: service_started
    environment:
      - MYSQL_USER=${MYSQL_USER}
      - MYSQL_HOST=${MYSQL_HOST}
      - MYSQL_PORT=3306
      - MYSQL_DATABASE=${MYSQL_DATABASE}
      - MYSQL_PASSWORD=\${MYSQL_APP_PASSWORD}
      - PORT=${PORT}
      - REQUEST_REGISTRATOR_URL=${REQUEST_REGISTRATOR_URL}
      - ORCHESTRATOR_URL=${ORCHESTRATOR_URL}
      - POLL_FREQUENCY_SECS=${POLL_FREQUENCY_SECS}
    ports:
      - 8000:8000
    logging:
      driver: "local"
      options:
        max-size: 100m
        max-file: "15"

  vamp_fun_db:
    container_name: vamp_fun_db
    image: ${DB_DOCKER_IMAGE}
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=\${MYSQL_ROOT_PASSWORD}
      - MYSQL_APP_PASSWORD=\${MYSQL_APP_PASSWORD}
      - MYSQL_READER_PASSWORD=\${MYSQL_READER_PASSWORD}
    volumes:
      - mysql:/var/lib/mysql
    ports:
      - 3306:3306

  vamp_fun_request_registrator:
    container_name: vamp_fun_request_registrator
    image: request-registrator-updated-image
    restart: unless-stopped
    depends_on:
      vamp_fun_redis:
        condition: service_started
    ports:
      - 50051:50051

  vamp_fun_orchestrator:
    container_name: vamp_fun_orchestrator
    image: orchestrator-updated-image
    restart: unless-stopped
    depends_on:
      vamp_fun_redis:
        condition: service_started
    environment:
      - SOLANA_PRIVATE_KEY=\${SOLANA_PRIVATE_KEY}
    ports:
      - 50052:50052

  vamp_fun_redis:
    container_name: vamp_fun_redis
    image: ${REDIS_DOCKER_IMAGE}
    restart: unless-stopped
    ports:
      - 6379:6379

volumes:
  mysql:

COMPOSE

set -e

# Pull images:
docker pull ${SOLVER_DOCKER_IMAGE}
docker pull ${DB_DOCKER_IMAGE}
docker pull ${ORCHESTRATOR_DOCKER_IMAGE}
docker pull ${REQUEST_REGISTRATOR_DOCKER_IMAGE}
docker pull ${REDIS_DOCKER_IMAGE}

# Push configs into docker images.
# Request registrator
cat >request_registrator_config.toml << REQUEST_REGISTRATOR_CONFIG
[ethereum]
rpc_url = "${REQUEST_REGISTRATOR_ETHEREUM_RPC_URL}"
contract_address = "${REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS}"

[grpc]
address = "${REQUEST_REGISTRATOR_GRPC_ADDRESS}"

[storage]
redis_url = "${REQUEST_REGISTRATOR_STORAGE_REDIS_URL}"

REQUEST_REGISTRATOR_CONFIG

TMP_CONTAINER=$(docker create --name request-registrator-temp-container ${REQUEST_REGISTRATOR_DOCKER_IMAGE})
docker cp request_registrator_config.toml request-registrator-temp-container:/config/config.toml
docker commit request-registrator-temp-container request-registrator-updated-image
docker rm ${TMP_CONTAINER}
rm request_registrator_config.toml

# Orchestrator
cat >orchestrator_config.toml << ORCHESTRATOR_CONFIG
[solana]
cluster = "${ORCHESTRATOR_SOLANA_CLUSTER}"
program_address = "${ORCHESTRATOR_SOLANA_PROGRAM_ADDRESS}"

[grpc]
address = "${ORCHESTRATOR_GRPC_ADDRESS}"

[storage]
redis_url = "${ORCHESTRATOR_STORAGE_REDIS_URL}"

ORCHESTRATOR_CONFIG

TMP_CONTAINER=$(docker create --name orchestrator-temp-container ${ORCHESTRATOR_DOCKER_IMAGE})
docker cp orchestrator_config.toml orchestrator-temp-container:/config/orchestrator.toml
docker commit orchestrator-temp-container orchestrator-updated-image
docker rm ${TMP_CONTAINER}
rm orchestrator_config.toml

# Start our docker images.
./up.sh
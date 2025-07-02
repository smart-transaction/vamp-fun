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
        REQUEST_REGISTRATOR_URL="http://vamp_fun_request_registrator_ethereum:50051"
        ORCHESTRATOR_URL="http://vamp_fun_orchestrator:50052"
        VALIDATOR_URL="http://vamp_fun_validator_vamp:50053"
        POLL_FREQUENCY_SECS=5
        ETHEREUM_RPC_URL_WSS="wss://service.lestnet.org:8888"
        BASE_RPC_URL_WSS="wss://service.lestnet.org:8888"
        POLYGON_RPC_URL_WSS="wss://service.lestnet.org:8888"
        ARBITRUM_RPC_URL_WSS="wss://service.lestnet.org:8888"
        REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS="0x4e01a97f540D830b27F0b31Bd7eB1B477b7B6710"
        REQUEST_REGISTRATOR_GRPC_ADDRESS="[::]:50051"
        REQUEST_REGISTRATOR_STORAGE_REDIS_URL="redis://vamp_fun_redis:6379"
        BASE_RPC_URL_WSS="wss://service.lestnet.org:8888"
        QUICKNODE_API_KEY=$(gcloud secrets versions access 1 --secret="QUICKNODE_API_KEY")
        ORCHESTRATOR_SOLANA_DEVNET_URL="https://red-burned-rain.solana-devnet.quiknode.pro/${QUICKNODE_API_KEY}"
        ORCHESTRATOR_SOLANA_MAINNET_URL="${ORCHESTRATOR_SOLANA_DEVNET_URL}"
        ORCHESTRATOR_SOLANA_DEFAULT_URL="${ORCHESTRATOR_SOLANA_DEVNET_URL}"
        ORCHESTRATOR_GRPC_ADDRESS="[::]:50052"
        ORCHESTRATOR_STORAGE_REDIS_URL="redis://vamp_fun_redis:6379"
        VALIDATOR_GRPC_ADDRESS="[::]:50053"
        VALIDATOR_STORAGE_REDIS_URL="redis://vamp_fun_redis:6379"
        VALIDATOR_IPFS_API_URL="http://vampfun-dev.stxn.io:5001"
        VALIDATOR_IPFS_GATEWAY_URL="https://ipfs.io/"
        DEFAULT_SOLANA_CLUSTER="DEVNET"
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
        REQUEST_REGISTRATOR_URL="http://vamp_fun_request_registrator_ethereum:50051"
        ORCHESTRATOR_URL="http://vamp_fun_orchestrator:50052"
        VALIDATOR_URL="http://vamp_fun_validator_vamp:50053"
        POLL_FREQUENCY_SECS=5
        REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS="0x672bbf1E4bEdC6Cce3FD1c1B47883641FcAB5088"
        REQUEST_REGISTRATOR_GRPC_ADDRESS="[::]:50051"
        REQUEST_REGISTRATOR_STORAGE_REDIS_URL="redis://vamp_fun_redis:6379"
        QUICKNODE_API_KEY=$(gcloud secrets versions access 1 --secret="QUICKNODE_API_KEY")
        ETHEREUM_RPC_URL_WSS="wss://red-burned-rain.quiknode.pro/${QUICKNODE_API_KEY}"
        BASE_RPC_URL_WSS="wss://red-burned-rain.base-mainnet.quiknode.pro/${QUICKNODE_API_KEY}"
        POLYGON_RPC_URL_WSS="wss://red-burned-rain.matic.quiknode.pro/${QUICKNODE_API_KEY}"
        ARBITRUM_RPC_URL_WSS="wss://red-burned-rain.arbitrum-mainnet.quiknode.pro/${QUICKNODE_API_KEY}"
        ORCHESTRATOR_SOLANA_DEVNET_URL="https://red-burned-rain.solana-devnet.quiknode.pro/${QUICKNODE_API_KEY}"
        ORCHESTRATOR_SOLANA_MAINNET_URL="https://red-burned-rain.solana-mainnet.quiknode.pro/${QUICKNODE_API_KEY}"
        ORCHESTRATOR_SOLANA_DEFAULT_URL="${ORCHESTRATOR_SOLANA_MAINNET_URL}"
        ORCHESTRATOR_GRPC_ADDRESS="[::]:50052"
        ORCHESTRATOR_STORAGE_REDIS_URL="redis://vamp_fun_redis:6379"
        VALIDATOR_GRPC_ADDRESS="[::]:50053"
        VALIDATOR_STORAGE_REDIS_URL="redis://vamp_fun_redis:6379"
        VALIDATOR_IPFS_API_URL="http://vampfun-prod.stxn.io:5001"
        VALIDATOR_IPFS_GATEWAY_URL="https://ipfs.io/"
        DEFAULT_SOLANA_CLUSTER="MAINNET"
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
SOLANA_PRIVATE_KEY=\$(gcloud secrets versions access 2 --secret="VAMP_FUN_SOLANA_PRIVATE_KEY_${SECRET_SUFFIX}")
SOLVER_PRIVATE_KEY=\$(gcloud secrets versions access 1 --secret="VAMP_FUN_SOLVER_PRIVATE_KEY_${SECRET_SUFFIX}")
VALIDATOR_PRIVATE_KEY=\$(gcloud secrets versions access 1 --secret="VAMP_FUN_VALIDATOR_PRIVATE_KEY_${SECRET_SUFFIX}")

ENV

docker compose up -d --remove-orphans

rm -f .env

UP

sudo chmod a+x up.sh

cat >down.sh << DOWN
# Turn down solver.
docker compose down
DOWN
sudo chmod a+x down.sh

# Docker images
DOCKER_LOCATION="us-central1-docker.pkg.dev"
DOCKER_PREFIX="${DOCKER_LOCATION}/solver-438012/solver-docker-repo"
SOLVER_DOCKER_IMAGE="${DOCKER_PREFIX}/vampfun-solver-image:${OPT}"
DB_DOCKER_IMAGE="${DOCKER_PREFIX}/vampfun-db-image:live"
ORCHESTRATOR_DOCKER_IMAGE="${DOCKER_PREFIX}/vampfun-orchestrator-image:${OPT}"
REQUEST_REGISTRATOR_DOCKER_IMAGE="${DOCKER_PREFIX}/vampfun-request-registrator-image:${OPT}"
VALIDATOR_VAMP_DOCKER_IMAGE="${DOCKER_PREFIX}/vampfun-validator-vamp-image:${OPT}"
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
      vamp_fun_request_registrator_ethereum:
        condition: service_started
      vamp_fun_request_registrator_base:
        condition: service_started
      vamp_fun_request_registrator_polygon:
        condition: service_started
      vamp_fun_request_registrator_arbitrum:
        condition: service_started
      vamp_fun_orchestrator:
        condition: service_started
      vamp_fun_validator_vamp:
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
      - VALIDATOR_URL=${VALIDATOR_URL}
      - POLL_FREQUENCY_SECS=${POLL_FREQUENCY_SECS}
      - QUICKNODE_API_KEY=${QUICKNODE_API_KEY}
      - SOLVER_PRIVATE_KEY=\${SOLVER_PRIVATE_KEY}
      - SOLANA_PRIVATE_KEY=\${SOLANA_PRIVATE_KEY}
      - DEFAULT_SOLANA_CLUSTER=${DEFAULT_SOLANA_CLUSTER}
    ports:
      - 8000:8000
    logging:
      driver: "json-file"
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

  # Ethereum + only single exposed grpc rr
  vamp_fun_request_registrator_ethereum:
    container_name: vamp_fun_request_registrator_ethereum
    image: request-registrator-ethereum-updated-image
    restart: unless-stopped
    depends_on:
      vamp_fun_redis:
        condition: service_started
    ports:
      - 50051:50051
    logging:
      driver: "json-file"
      options:
        max-size: 100m
        max-file: "15"

  vamp_fun_request_registrator_base:
    container_name: vamp_fun_request_registrator_base
    image: request-registrator-base-updated-image
    restart: unless-stopped
    depends_on:
      vamp_fun_redis:
        condition: service_started
    logging:
      driver: "json-file"
      options:
        max-size: 100m
        max-file: "15"

  vamp_fun_request_registrator_polygon:
    container_name: vamp_fun_request_registrator_polygon
    image: request-registrator-polygon-updated-image
    restart: unless-stopped
    depends_on:
      vamp_fun_redis:
        condition: service_started
    logging:
      driver: "json-file"
      options:
        max-size: 100m
        max-file: "15"

  vamp_fun_request_registrator_arbitrum:
    container_name: vamp_fun_request_registrator_arbitrum
    image: request-registrator-arbitrum-updated-image
    restart: unless-stopped
    depends_on:
      vamp_fun_redis:
        condition: service_started
    logging:
      driver: "json-file"
      options:
        max-size: 100m
        max-file: "15"

  vamp_fun_orchestrator:
    container_name: vamp_fun_orchestrator
    image: orchestrator-updated-image
    restart: unless-stopped
    depends_on:
      vamp_fun_redis:
        condition: service_started
    environment:
      - RUST_LOG=debug
    ports:
      - 50052:50052
    logging:
      driver: "json-file"
      options:
        max-size: 100m
        max-file: "15"

  vamp_fun_validator_vamp:
    container_name: vamp_fun_validator_vamp
    image: validator-vamp-updated-image
    restart: unless-stopped
    depends_on:
      vamp_fun_redis:
        condition: service_started
    environment:
      - RUST_LOG=debug
      - VALIDATOR_PRIVATE_KEY=\${VALIDATOR_PRIVATE_KEY}
    ports:
      - 50053:50053
    logging:
      driver: "json-file"
      options:
        max-size: 100m
        max-file: "15"

  vamp_fun_redis:
    container_name: vamp_fun_redis
    image: ${REDIS_DOCKER_IMAGE}
    restart: unless-stopped
    ports:
      - 6379:6379
    volumes:
      - redis-data:/data

volumes:
  mysql:
    name: vamp_fun_mysql
    external: false
  redis-data:
    # Supposed to be managed by docker and not be flushed on "docker-compose down"
    # Should only be dropped with "docker-compose down -v"
    name: vamp_fun_redis_data
    external: false

COMPOSE

set -e

# Pull images:
docker pull ${SOLVER_DOCKER_IMAGE}
docker pull ${DB_DOCKER_IMAGE}
docker pull ${ORCHESTRATOR_DOCKER_IMAGE}
docker pull ${REQUEST_REGISTRATOR_DOCKER_IMAGE}
docker pull ${VALIDATOR_VAMP_DOCKER_IMAGE}
docker pull ${REDIS_DOCKER_IMAGE}

# Push configs into docker images.
# Request registrator
# Ethereum + only single exposed grpc rr
# TODO-KG: Some loop would be nice here :)
# TODO-KG: If XXX__RPC_URL_WSS is empty - skip the deployment (also skip it in compose)
cat >request_registrator_config.toml << REQUEST_REGISTRATOR_CONFIG
[ethereum]
rpc_url = "${ETHEREUM_RPC_URL_WSS}"
contract_address = "${REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS}"

[grpc]
address = "${REQUEST_REGISTRATOR_GRPC_ADDRESS}"

[storage]
redis_url = "${REQUEST_REGISTRATOR_STORAGE_REDIS_URL}"

REQUEST_REGISTRATOR_CONFIG

TMP_CONTAINER=$(docker create --name request-registrator-temp-container ${REQUEST_REGISTRATOR_DOCKER_IMAGE})
docker cp request_registrator_config.toml request-registrator-temp-container:/config/config.toml
docker commit request-registrator-temp-container request-registrator-ethereum-updated-image
docker rm ${TMP_CONTAINER}
rm request_registrator_config.toml

# Base
cat >request_registrator_config.toml << REQUEST_REGISTRATOR_CONFIG
[ethereum]
rpc_url = "${BASE_RPC_URL_WSS}"
contract_address = "${REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS}"

[grpc]
address = "${REQUEST_REGISTRATOR_GRPC_ADDRESS}"

[storage]
redis_url = "${REQUEST_REGISTRATOR_STORAGE_REDIS_URL}"

REQUEST_REGISTRATOR_CONFIG

TMP_CONTAINER=$(docker create --name request-registrator-temp-container ${REQUEST_REGISTRATOR_DOCKER_IMAGE})
docker cp request_registrator_config.toml request-registrator-temp-container:/config/config.toml
docker commit request-registrator-temp-container request-registrator-base-updated-image
docker rm ${TMP_CONTAINER}
rm request_registrator_config.toml

# Polygon
cat >request_registrator_config.toml << REQUEST_REGISTRATOR_CONFIG
[ethereum]
rpc_url = "${POLYGON_RPC_URL_WSS}"
contract_address = "${REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS}"

[grpc]
address = "${REQUEST_REGISTRATOR_GRPC_ADDRESS}"

[storage]
redis_url = "${REQUEST_REGISTRATOR_STORAGE_REDIS_URL}"

REQUEST_REGISTRATOR_CONFIG

TMP_CONTAINER=$(docker create --name request-registrator-temp-container ${REQUEST_REGISTRATOR_DOCKER_IMAGE})
docker cp request_registrator_config.toml request-registrator-temp-container:/config/config.toml
docker commit request-registrator-temp-container request-registrator-polygon-updated-image
docker rm ${TMP_CONTAINER}
rm request_registrator_config.toml

# Arbitrum
cat >request_registrator_config.toml << REQUEST_REGISTRATOR_CONFIG
[ethereum]
rpc_url = "${ARBITRUM_RPC_URL_WSS}"
contract_address = "${REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS}"

[grpc]
address = "${REQUEST_REGISTRATOR_GRPC_ADDRESS}"

[storage]
redis_url = "${REQUEST_REGISTRATOR_STORAGE_REDIS_URL}"

REQUEST_REGISTRATOR_CONFIG

TMP_CONTAINER=$(docker create --name request-registrator-temp-container ${REQUEST_REGISTRATOR_DOCKER_IMAGE})
docker cp request_registrator_config.toml request-registrator-temp-container:/config/config.toml
docker commit request-registrator-temp-container request-registrator-arbitrum-updated-image
docker rm ${TMP_CONTAINER}
rm request_registrator_config.toml

# Orchestrator
cat >orchestrator_config.toml << ORCHESTRATOR_CONFIG
[solana]
devnet_url = "${ORCHESTRATOR_SOLANA_DEVNET_URL}"
mainnet_url = "${ORCHESTRATOR_SOLANA_MAINNET_URL}"
default_url = "${ORCHESTRATOR_SOLANA_DEFAULT_URL}"

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

# Validator Vamp
cat >validator_vamp_config.toml << VALIDATOR_VAMP_CONFIG
[grpc]
binding_url = "${VALIDATOR_GRPC_ADDRESS}"

[storage]
redis_url = "${VALIDATOR_STORAGE_REDIS_URL}"

[ipfs]
api_url = "${VALIDATOR_IPFS_API_URL}"
gateway_url = "${VALIDATOR_IPFS_GATEWAY_URL}"
pin = true
enable_mfs_copy = true

VALIDATOR_VAMP_CONFIG

echo "Created validator_vamp_config.toml:"
cat validator_vamp_config.toml

# Clean up any existing temporary container
docker rm -f validator-vamp-temp-container 2>/dev/null || true

TMP_CONTAINER=$(docker create --name validator-vamp-temp-container ${VALIDATOR_VAMP_DOCKER_IMAGE})
docker cp validator_vamp_config.toml validator-vamp-temp-container:/config/validator_vamp_config.toml

docker commit --change='CMD ["validator_vamp", "/config/validator_vamp_config.toml"]' validator-vamp-temp-container validator-vamp-updated-image
docker rm ${TMP_CONTAINER}
rm validator_vamp_config.toml

# Start our docker images.
./up.sh
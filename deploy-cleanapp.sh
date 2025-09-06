# Cleanapp-focused STXN infra setup (Redis + Request Registrator)
# Later we will add solver, validator, orchestrator.

set -e

# Optional CLI arg: -e|--env <dev|prod>
OPT=""
while [[ $# -gt 0 ]]; do
  case $1 in
    "-e"|"--env")
      OPT="$2"
      shift 2
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

if [ -z "${OPT}" ]; then
  PS3="Please choose the environment: "
  options=("dev" "prod" "quit")
  select OPT in "${options[@]}"
  do
    case ${OPT} in
      "dev")
          echo "Using dev environment"
          REQUEST_REGISTRATOR_GRPC_ADDRESS="[::]:50051"
          REQUEST_REGISTRATOR_STORAGE_REDIS_URL="redis://cleanapp_stxn_redis:6379"
          # For now no on-chain listener; RR will only accept gRPC Push
          ETHEREUM_RPC_URL_WSS="wss://service.lestnet.org:8888"
          REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS="0x4e01a97f540D830b27F0b31Bd7eB1B477b7B6710"
          # Orchestrator EVM endpoints map (chainId -> RPC URL)
          # Dev: eip155:21363 at lestnet service
          EVM_ENDPOINT_21363="wss://service.lestnet.org:8888"
          break
          ;;
      "prod")
          echo "Using prod environment"
          REQUEST_REGISTRATOR_GRPC_ADDRESS="[::]:50051"
          REQUEST_REGISTRATOR_STORAGE_REDIS_URL="redis://cleanapp_stxn_redis:6379"
          ETHEREUM_RPC_URL_WSS=""
          REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS=""
          EVM_ENDPOINT_21363="wss://service.lestnet.org:8888"
          break
          ;;
      "quit")
          exit
          ;;
      *) echo "invalid option $REPLY";;
    esac
  done
else
  case ${OPT} in
    "dev")
        echo "Using dev environment"
        REQUEST_REGISTRATOR_GRPC_ADDRESS="[::]:50051"
        REQUEST_REGISTRATOR_STORAGE_REDIS_URL="redis://cleanapp_stxn_redis:6379"
        ETHEREUM_RPC_URL_WSS="wss://service.lestnet.org:8888"
        REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS="0x4e01a97f540D830b27F0b31Bd7eB1B477b7B6710"
        EVM_ENDPOINT_21363="wss://service.lestnet.org:8888"
        ;;
    "prod")
        echo "Using prod environment"
        REQUEST_REGISTRATOR_GRPC_ADDRESS="[::]:50051"
        REQUEST_REGISTRATOR_STORAGE_REDIS_URL="redis://cleanapp_stxn_redis:6379"
        ETHEREUM_RPC_URL_WSS=""
        REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS=""
        EVM_ENDPOINT_21363="wss://service.lestnet.org:8888"
        ;;
    *)
        echo "Unknown environment: ${OPT}. Use dev|prod"
        exit 1
        ;;
  esac
fi

# Obtain secrets
SECRET_SUFFIX=$(echo ${OPT} | tr '[a-z]' '[A-Z]')
SOLVER_PRIVATE_KEY=$(gcloud secrets versions access 1 --secret="VAMP_FUN_SOLVER_PRIVATE_KEY_${SECRET_SUFFIX}")

# Create up/down helpers
cat >up.sh << UP
set -e

docker compose up -d --remove-orphans
UP
chmod +x up.sh

cat >down.sh << DOWN
set -e
docker compose down
DOWN
chmod +x down.sh

# Docker images
DOCKER_LOCATION="us-central1-docker.pkg.dev"
DOCKER_PREFIX="${DOCKER_LOCATION}/solver-438012/solver-docker-repo"
REQUEST_REGISTRATOR_DOCKER_IMAGE="
${DOCKER_PREFIX}/vampfun-request-registrator-image:${OPT}"
ORCHESTRATOR_DOCKER_IMAGE="${DOCKER_PREFIX}/vampfun-orchestrator-image:${OPT}"
SOLVER_CLEANAPP_DOCKER_IMAGE="${DOCKER_PREFIX}/vampfun-solver-cleanapp-image:${OPT}"
REDIS_DOCKER_IMAGE=redis/redis-stack-server:latest

# Compose file (redis + registrator + orchestrator placeholder + solver)
cat >docker-compose.yml << COMPOSE
services:
  cleanapp_stxn_request_registrator:
    container_name: cleanapp_stxn_request_registrator
    image: request-registrator-cleanapp-updated-image
    restart: unless-stopped
    depends_on:
      cleanapp_stxn_redis:
        condition: service_started
    ports:
      - 50051:50051
    logging:
      driver: "json-file"
      options:
        max-size: 100m
        max-file: "15"

  cleanapp_stxn_orchestrator:
    container_name: cleanapp_stxn_orchestrator
    image: orchestrator-cleanapp-updated-image
    restart: unless-stopped
    depends_on:
      cleanapp_stxn_redis:
        condition: service_started
    ports:
      - 50052:50052

  cleanapp_solver_cleanapp:
    container_name: cleanapp_solver_cleanapp
    image: \
${SOLVER_CLEANAPP_DOCKER_IMAGE}
    restart: unless-stopped
    depends_on:
      cleanapp_stxn_request_registrator:
        condition: service_started
      cleanapp_stxn_orchestrator:
        condition: service_started
    environment:
      - REQUEST_REGISTRATOR_URL=http://cleanapp_stxn_request_registrator:50051
      - ORCHESTRATOR_URL=http://cleanapp_stxn_orchestrator:50052
      - POLL_FREQUENCY=5s
      - EVM_PRIVATE_KEY_HEX=${SOLVER_PRIVATE_KEY}
      - ERC20_TOKEN_ADDRESS=0x0000000000000000000000000000000000000000
      - AMOUNT_WEI=0
      - EIP155_CHAIN_REF=21363

  cleanapp_stxn_redis:
    container_name: cleanapp_stxn_redis
    image: ${REDIS_DOCKER_IMAGE}
    restart: unless-stopped
    ports:
      - 6379:6379
    volumes:
      - redis-data:/data

volumes:
  redis-data:
    name: cleanapp_stxn_redis_data
    external: false
COMPOSE

# Pull images
set -e
docker pull ${REQUEST_REGISTRATOR_DOCKER_IMAGE}
docker pull ${ORCHESTRATOR_DOCKER_IMAGE}
docker pull ${SOLVER_CLEANAPP_DOCKER_IMAGE}
docker pull ${REDIS_DOCKER_IMAGE}

# Prepare RR config (gRPC binding + storage only; ethereum section left blank)
cat >request_registrator_config.toml << REQUEST_REGISTRATOR_CONFIG
[ethereum]
rpc_url = "${ETHEREUM_RPC_URL_WSS}"
contract_address = "${REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS}"

[grpc]
address = "${REQUEST_REGISTRATOR_GRPC_ADDRESS}"

[storage]
redis_url = "${REQUEST_REGISTRATOR_STORAGE_REDIS_URL}"
REQUEST_REGISTRATOR_CONFIG

# Bake configs into images: request-registrator + orchestrator
TMP_CONTAINER=$(docker create --name request-registrator-temp-container ${REQUEST_REGISTRATOR_DOCKER_IMAGE})
docker cp request_registrator_config.toml request-registrator-temp-container:/config/config.toml
docker commit request-registrator-temp-container request-registrator-cleanapp-updated-image
docker rm ${TMP_CONTAINER}
rm request_registrator_config.toml

# Orchestrator config with EVM endpoints
cat >orchestrator_config.toml << ORCHESTRATOR_CONFIG
[solana]
devnet_url = ""
mainnet_url = ""
default_url = ""

[grpc]
address = "[::]:50052"

[evm.endpoints]
"21363" = "${EVM_ENDPOINT_21363}"

[storage]
redis_url = "redis://cleanapp_stxn_redis:6379"
ORCHESTRATOR_CONFIG

TMP_CONTAINER=$(docker create --name orchestrator-cleanapp-temp-container ${ORCHESTRATOR_DOCKER_IMAGE})
docker cp orchestrator_config.toml orchestrator-cleanapp-temp-container:/config/orchestrator.toml
docker commit orchestrator-cleanapp-temp-container orchestrator-cleanapp-updated-image
docker rm ${TMP_CONTAINER}
rm orchestrator_config.toml

# Start services
./up.sh 
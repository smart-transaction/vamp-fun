# Cleanapp-focused STXN infra setup (Redis + Request Registrator)
# Later we will add solver, validator, orchestrator.

set -e

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
        ETHEREUM_RPC_URL_WSS=""
        REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS=""
        break
        ;;
    "prod")
        echo "Using prod environment"
        REQUEST_REGISTRATOR_GRPC_ADDRESS="[::]:50051"
        REQUEST_REGISTRATOR_STORAGE_REDIS_URL="redis://cleanapp_stxn_redis:6379"
        ETHEREUM_RPC_URL_WSS=""
        REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS=""
        break
        ;;
    "quit")
        exit
        ;;
    *) echo "invalid option $REPLY";;
  esac
done

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
REDIS_DOCKER_IMAGE=redis/redis-stack-server:latest

# Compose file (only redis + one registrator)
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

# Bake config into image to produce request-registrator-cleanapp-updated-image
TMP_CONTAINER=$(docker create --name request-registrator-temp-container ${REQUEST_REGISTRATOR_DOCKER_IMAGE})
docker cp request_registrator_config.toml request-registrator-temp-container:/config/config.toml
docker commit request-registrator-temp-container request-registrator-cleanapp-updated-image
docker rm ${TMP_CONTAINER}
rm request_registrator_config.toml

# Start services
./up.sh 
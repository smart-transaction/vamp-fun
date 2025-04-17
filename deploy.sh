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
        REQUEST_REGISTRATOR_URL=
        ORCHESTRATOR_URL=
        POLL_FREQUENCY_SECS=5
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
        REQUEST_REGISTRATOR_URL=
        ORCHESTRATOR_URL=
        POLL_FREQUENCY_SECS=5
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

# Create docker-compose.yml file.
cat >docker-compose.yml << COMPOSE
version: '3'

services:
  vamp_fun_solver:
    container_name: vamp_fun_solver
    image: ${SOLVER_DOCKER_IMAGE}
    restart: unless-stopped
    depends_on:
      vamp_fun_db:
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

volumes:
  mysql:

COMPOSE

set -e

# Pull images:
docker pull ${SOLVER_DOCKER_IMAGE}
docker pull ${DB_DOCKER_IMAGE}

# Start our docker images.
./up.sh
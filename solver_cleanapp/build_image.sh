#!/bin/bash

echo "Building solver-cleanapp docker image..."

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
  select OPT in "${options[@]}"; do
    case ${OPT} in
      "dev"|"prod") break;;
      "quit") exit;;
      *) echo "invalid option $REPLY";;
    esac
  done
fi

echo "Using ${OPT} environment"

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cd "${SCRIPT_DIR}"

# Get current git commit
if git rev-parse HEAD >/dev/null 2>&1; then
  GIT_COMMIT=$(git rev-parse HEAD)
  GIT_COMMIT_SHORT=$(git rev-parse --short HEAD)
  echo "Using git commit: ${GIT_COMMIT_SHORT} (${GIT_COMMIT})"
else
  echo "Warning: Not in a git repository, using 'unknown' for git commit"
  GIT_COMMIT="unknown"
fi

test -d target && rm -rf target

# Ensure .version exists
if [ ! -f .version ]; then
  echo "BUILD_VERSION=1.0.0" > .version
fi
. .version

# Increment version build number for dev
if [ "${OPT}" == "dev" ]; then
  BUILD=$(echo ${BUILD_VERSION} | cut -f 3 -d ".")
  VER=$(echo ${BUILD_VERSION} | cut -f 1,2 -d ".")
  BUILD=$((${BUILD} + 1))
  BUILD_VERSION="${VER}.${BUILD}"
  echo "BUILD_VERSION=${BUILD_VERSION}" > .version
fi

echo "Running docker build for version ${BUILD_VERSION}"

set -e

CLOUD_REGION="us-central1"
PROJECT_NAME="solver-438012"
DOCKER_IMAGE="solver-docker-repo/vampfun-solver-cleanapp-image"
DOCKER_TAG="${CLOUD_REGION}-docker.pkg.dev/${PROJECT_NAME}/${DOCKER_IMAGE}"

CURRENT_PROJECT=$(gcloud config get project)
echo ${CURRENT_PROJECT}
if [ "${PROJECT_NAME}" != "${CURRENT_PROJECT}" ]; then
  gcloud auth login
  gcloud config set project ${PROJECT_NAME}
fi

if [ "${OPT}" == "dev" ]; then
  echo "Building and pushing docker image via Cloud Build..."
  mkdir -p target/solver_cleanapp
  # Stage component context: copy self plus shared proto into local proto/
  rsync -a --delete --exclude target ./ target/solver_cleanapp/
  rsync -a ../proto/ target/solver_cleanapp/proto/
  # Submit cloud build with upper-level vamp-fun/cloudbuild.yaml and staged context
  gcloud builds submit \
    --region=${CLOUD_REGION} \
    --substitutions=_TAG=${DOCKER_TAG}:${BUILD_VERSION},_GIT_COMMIT=${GIT_COMMIT} \
    --config ${SCRIPT_DIR}/../cloudbuild.yaml \
    ${SCRIPT_DIR}/target/solver_cleanapp
fi

echo "Tagging Docker image as current ${OPT}..."
gcloud artifacts docker tags add ${DOCKER_TAG}:${BUILD_VERSION} ${DOCKER_TAG}:${OPT}

echo "solver-cleanapp docker image build completed successfully!" 
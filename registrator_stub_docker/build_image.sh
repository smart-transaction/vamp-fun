echo "Building request registrator docker image..."

# Choose the environment
PS3="Please choose the environment: "
options=("dev" "prod" "quit")
select OPT in "${options[@]}"
do
  case ${OPT} in
    "dev")
        echo "Using dev environment"
        break
        ;;
    "prod")
        echo "Using prod environment"
        break
        ;;
    "quit")
        exit
        ;;
    *) echo "invalid option $REPLY";;
  esac
done

# Get current git commit (this should be run from the git repository)
if git rev-parse HEAD >/dev/null 2>&1; then
    GIT_COMMIT=$(git rev-parse HEAD)
    GIT_COMMIT_SHORT=$(git rev-parse --short HEAD)
    echo "Using git commit: ${GIT_COMMIT_SHORT} (${GIT_COMMIT})"
else
    echo "Warning: Not in a git repository, using 'unknown' for git commit"
    GIT_COMMIT="unknown"
fi

test -d target && rm -rf target

. .version

# Increment version build number
if [ "${OPT}" == "dev" ]; then
  BUILD=$(echo ${BUILD_VERSION} | cut -f 3 -d ".")
  VER=$(echo ${BUILD_VERSION} | cut -f 1,2 -d ".")
  BUILD=$((${BUILD} + 1))
  BUILD_VERSION="${VER}.${BUILD}"
  echo "BUILD_VERSION=${BUILD_VERSION}" > .version
  pushd ../registrator_stub
  cargo set-version ${BUILD_VERSION}
  popd
fi

echo "Running docker build for version ${BUILD_VERSION}"

set -e

CLOUD_REGION="us-central1"
PROJECT_NAME="solver-438012"
DOCKER_IMAGE="solver-docker-repo/vampfun-request-registrator-image"
DOCKER_TAG="${CLOUD_REGION}-docker.pkg.dev/${PROJECT_NAME}/${DOCKER_IMAGE}"

CURRENT_PROJECT=$(gcloud config get project)
echo ${CURRENT_PROJECT}
if [ "${PROJECT_NAME}" != "${CURRENT_PROJECT}" ]; then
  gcloud auth login
  gcloud config set project ${PROJECT_NAME}
fi

if [ "${OPT}" == "dev" ]; then
  echo "Building and pushing docker image..."
  mkdir -p target
  cp -rf ../registrator_stub target
  cp -rf ../proto target
  cp -rf ../abis target
  cp -rf ../appchain target
  rm -rf target/registrator_stub/target
  gcloud builds submit \
    --region=${CLOUD_REGION} \
    --substitutions=_TAG=${DOCKER_TAG}:${BUILD_VERSION},_GIT_COMMIT=${GIT_COMMIT} \
    --config ../cloudbuild.yaml
fi

echo "Tagging Docker image as current ${OPT}..."
gcloud artifacts docker tags add ${DOCKER_TAG}:${BUILD_VERSION} ${DOCKER_TAG}:${OPT}
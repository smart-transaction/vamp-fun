echo "Buiding solver docker image..."

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

test -d target && rm -rf target

. .version

# Increment version build number
if [ "${OPT}" == "dev" ]; then
  BUILD=$(echo ${BUILD_VERSION} | cut -f 3 -d ".")
  VER=$(echo ${BUILD_VERSION} | cut -f 1,2 -d ".")
  BUILD=$((${BUILD} + 1))
  BUILD_VERSION="${VER}.${BUILD}"
  echo "BUILD_VERSION=${BUILD_VERSION}" > .version
  pushd ../solver
  cargo set-version ${BUILD_VERSION}
  popd
fi

echo "Running docker build for version ${BUILD_VERSION}"

set -e

CLOUD_REGION="us-central1"
PROJECT_NAME="solver-438012"
DOCKER_IMAGE="solver-docker-repo/vampfun-solver-image"
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
  cp -rf ../solver target
  cp -rf ../proto target
  cp -rf ../crates target
  cp -rf ../idls target
  rm -rf target/solver/target
  rm -rf target/crates/**/target
  gcloud builds submit \
    --region=${CLOUD_REGION} \
    --substitutions=_TAG=${DOCKER_TAG}:${BUILD_VERSION} \
    --config ../cloudbuild.yaml
fi

echo "Tagging Docker image as current ${OPT}..."
gcloud artifacts docker tags add ${DOCKER_TAG}:${BUILD_VERSION} ${DOCKER_TAG}:${OPT}
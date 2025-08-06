#!/bin/bash

echo "Testing frontend docker build locally..."

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

echo "Running local docker build for version ${BUILD_VERSION}"

set -e

# Create target directory and copy frontend
mkdir -p target

# Copy frontend repository to target
cp -rf ../../vamp-fun-fe/* target/

# Switch to appropriate branch based on environment
pushd ../../vamp-fun-fe
if [ "${OPT}" == "dev" ]; then
  git checkout dev
else
  git checkout main
fi
popd

# Copy again after branch switch
cp -rf ../../vamp-fun-fe/* target/

# Remove git directory to avoid copying it
rm -rf target/.git

# Build docker image locally
docker build \
  --build-arg GIT_COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "unknown") \
  --build-arg NEXT_PUBLIC_PROJECT_ID="c3b36688e5b8c0314a3dc023ae6993c6" \
  --build-arg ALCHEMY_API_KEY="8--9cFBklER0CvBMnrdMYwMzQZzvWSds" \
  --build-arg SOLVER_TESTNET_URL="https://34-36-3-154.nip.io" \
  --build-arg SOLVER_MAINNET_URL="https://api.solver.vamp.stxn.io" \
  --build-arg NEXT_PUBLIC_FORCE_SOLANA_NETWORK="devnet" \
  -t vampfun-fe:${OPT} .

echo "Local build completed successfully!"
echo "You can test the image with: docker run -p 3000:3000 vampfun-fe:${OPT}" 
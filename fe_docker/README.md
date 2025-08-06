# Frontend Docker

This directory contains the Docker configuration for the Vamp Fun frontend application.

## Overview

The frontend is a Next.js application that gets containerized and deployed using Google Cloud Build. The setup follows the same pattern as other services in this repository.

## Structure

- `Dockerfile` - Multi-stage Docker build for the Next.js application
- `build_image.sh` - Script to build and push the Docker image to Google Cloud
- `.version` - Version tracking file
- `README.md` - This file

## Building the Image

To build the frontend Docker image:

```bash
cd fe_docker
./build_image.sh
```

The script will:
1. Ask you to choose between `dev` and `prod` environments
2. Copy the frontend repository from `../vamp-fun-fe/` to the `target/` directory
3. Switch to the appropriate branch (`dev` for dev, `main` for prod)
4. Build and push the Docker image to Google Cloud Artifact Registry
5. Tag the image as the current environment

## Testing Locally

To test the build locally before pushing to Google Cloud:

```bash
cd fe_docker
./test_build.sh
```

This will build the Docker image locally and tag it as `vampfun-fe:dev` or `vampfun-fe:prod`. You can then test it with:

```bash
docker run -p 3000:3000 vampfun-fe:dev
```

## Environment Branches

- `dev` environment uses the `dev` branch from the frontend repository
- `prod` environment uses the `main` branch from the frontend repository

## Deployment

The frontend can be deployed using the main `deploy.sh` script in the root directory, which will include the frontend service in the docker-compose setup.

## Port Configuration

The frontend runs on port 3000 by default. This can be configured in the docker-compose.yml file when deploying.

## Build Process

1. **Builder Stage**: Installs dependencies and builds the Next.js application
2. **Runner Stage**: Creates a minimal production image with only the built application and production dependencies

The build command used is `yarn --frozen-lockfile install; yarn build` as specified in the render.com configuration. 
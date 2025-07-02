# Validator Vamp Docker

This directory contains the Docker configuration for the validator-vamp component.

## Files

- `Dockerfile` - Multi-stage Docker build for the validator-vamp service
- `build_image.sh` - Script to build and push the Docker image to Google Cloud
- `.version` - Version tracking file
- `certificates/` - Directory for SSL certificates (if needed)
- `target/` - Build artifacts directory (created during build)

## Building the Image

To build the Docker image:

```bash
./build_image.sh
```

This will prompt you to choose between "dev" and "prod" environments.

## Environment Variables

The validator-vamp service requires the following environment variables:

- `VALIDATOR_PRIVATE_KEY` - Private key for the validator wallet

## Configuration

The service expects a configuration file at `/config/validator_vamp_config.toml` when running in the container.

## Port

The service exposes port 50053 for gRPC communication. 
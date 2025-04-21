FROM rust:1.86 AS builder

RUN apt-get update \
   && DEBIAN_FRONTEND=noninteractive \
      apt-get install --no-install-recommends --assume-yes \
         protobuf-compiler

WORKDIR /usr/src/vamp-fun-registrator-stub
COPY target/registrator_stub registrator_stub
COPY target/proto proto
COPY target/abis abis
RUN cargo install --path ./registrator_stub

FROM --platform=linux/amd64 ubuntu:22.04

RUN echo 'APT::Install-Suggests "0";' >> /etc/apt/apt.conf.d/00-docker
RUN echo 'APT::Install-Recommends "0";' >> /etc/apt/apt.conf.d/00-docker
RUN DEBIAN_FRONTEND=noninteractive \
   apt-get update \
   && rm -rf /var/lib/apt/lists/*
 
USER root

RUN apt-get update
RUN apt-get install -y ca-certificates
RUN apt-get install -y gettext-base

# Copy certificates to connect to the ethereum network
COPY certificates/* /usr/local/share/ca-certificates/
RUN update-ca-certificates

COPY --from=builder /usr/local/cargo/bin/vamp_fun_registrator_stub /usr/local/bin/vamp_fun_registrator_stub

# Set environment variables
ENV REQUEST_REGISTRATOR_ETHEREUM_RPC_URL=${REQUEST_REGISTRATOR_ETHEREUM_RPC_URL}
ENV REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS=${REQUEST_REGISTRATOR_ETHEREUM_CONTRACT_ADDRESS}
ENV REQUEST_REGISTRATOR_GRPC_ADDRESS=${REQUEST_REGISTRATOR_GRPC_ADDRESS}
ENV REQUEST_REGISTRATOR_STORAGE_REDIS_URL=${REQUEST_REGISTRATOR_STORAGE_REDIS_URL}

# Copy the template into the container
RUN mkdir -p /usr/local/bin/config
COPY config/config_template.toml /usr/local/bin/config/config_template.toml

# Create final config by substituting env variables
RUN envsubst < /usr/local/bin/config/config_template.toml > /usr/local/bin/config/config.toml

EXPOSE 50051/tcp
CMD "vamp_fun_registrator_stub"
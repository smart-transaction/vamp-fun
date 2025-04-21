FROM rust:1.86 AS builder

RUN apt-get update \
   && DEBIAN_FRONTEND=noninteractive \
      apt-get install --no-install-recommends --assume-yes \
         protobuf-compiler

WORKDIR /usr/src/vamp-fun-orchestrator-stub
COPY target/orchestrator_stub orchestrator_stub
COPY target/proto proto
COPY target/idls idls
RUN cargo install --path ./orchestrator_stub

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

COPY --from=builder /usr/local/cargo/bin/vamp_fun_orchestrator_stub /usr/local/bin/vamp_fun_orchestrator_stub

# Set environment variables
ENV ORCHESTRATOR_SOLANA_RPC_URL=${ORCHESTRATOR_SOLANA_RPC_URL}
ENV ORCHESTRATOR_SOLANA_PROGRAM_ADDRESS=${ORCHESTRATOR_SOLANA_CONTRACT_ADDRESS}
ENV ORCHESTRATOR_GRPC_ADDRESS=${ORCHESTRATOR_GRPC_ADDRESS}
ENV ORCHESTRATOR_STORAGE_REDIS_URL=${ORCHESTRATOR_STORAGE_REDIS_URL}

# Copy the template into the container
RUN mkdir -p /usr/local/bin/config
COPY config/config_template.toml /usr/local/bin/config/config_template.toml

# Create final config by substituting env variables
RUN envsubst < /usr/local/bin/config/config_template.toml > /usr/local/bin/config/config.toml

EXPOSE 50052/tcp
CMD "vamp_fun_orchestrator_stub"
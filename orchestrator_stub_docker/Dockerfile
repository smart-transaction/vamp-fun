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

# Copy certificates to connect to the ethereum network
COPY certificates/* /usr/local/share/ca-certificates/
RUN update-ca-certificates

COPY --from=builder /usr/local/cargo/bin/vamp_fun_orchestrator_stub /usr/local/bin/vamp_fun_orchestrator_stub
RUN mkdir -p src
COPY --from=builder /usr/src/vamp-fun-orchestrator-stub/orchestrator_stub/src/generated src/generated

# Required for future copying of the config before running the container
RUN mkdir -p config

EXPOSE 50052
CMD "vamp_fun_orchestrator_stub" "--solana-private-key=${SOLANA_PRIVATE_KEY}"
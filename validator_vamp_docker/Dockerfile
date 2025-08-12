FROM rust:1.86 AS builder

RUN apt-get update \
   && DEBIAN_FRONTEND=noninteractive \
      apt-get install --no-install-recommends --assume-yes \
         protobuf-compiler

WORKDIR /usr/src/vamp-fun-validator-vamp
COPY target/validator_vamp validator_vamp
COPY target/proto proto
COPY target/idls idls
COPY target/appchain appchain
RUN cargo install --path ./validator_vamp

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

COPY --from=builder /usr/local/cargo/bin/validator_vamp /usr/local/bin/validator_vamp
RUN mkdir -p src
COPY --from=builder /usr/src/vamp-fun-validator-vamp/validator_vamp/src/generated src/generated

# Required for future copying of the config before running the container
RUN mkdir -p config

# Set working directory
WORKDIR /

EXPOSE 50053
CMD ["validator_vamp"] 
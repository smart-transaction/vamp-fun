FROM rust:1.86 AS builder

# Add build argument for git commit
ARG GIT_COMMIT=unknown
LABEL git.commit=${GIT_COMMIT}

RUN apt-get update \
   && DEBIAN_FRONTEND=noninteractive \
      apt-get install --no-install-recommends --assume-yes \
         protobuf-compiler

WORKDIR /usr/src/vamp-fun-solver
COPY target/solver solver
COPY target/proto proto
COPY target/crates crates
COPY target/idls idls
RUN cargo install --path ./solver

FROM --platform=linux/amd64 ubuntu:22.04

# Add build argument for git commit
ARG GIT_COMMIT=unknown
LABEL git.commit=${GIT_COMMIT}

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

COPY --from=builder /usr/local/cargo/bin/vamp-fun-solver /usr/local/bin/vamp-fun-solver

# Add environment variable for runtime access
ENV GIT_COMMIT=${GIT_COMMIT}

# Vamping configuration parameters
ENV PAID_CLAIMING_ENABLED=false
ENV USE_BONDING_CURVE=false
ENV CURVE_SLOPE=1
ENV BASE_PRICE=100
ENV MAX_PRICE=1000
ENV FLAT_PRICE_PER_TOKEN=1

EXPOSE 8000/tcp
CMD \
   "vamp-fun-solver" \
   "--port=${PORT}" \
   "--mysql-user=${MYSQL_USER}" \
   "--mysql-password=${MYSQL_PASSWORD}" \
   "--mysql-host=${MYSQL_HOST}" \
   "--mysql-port=${MYSQL_PORT}" \
   "--mysql-database=${MYSQL_DATABASE}" \
   "--request-registrator-url=${REQUEST_REGISTRATOR_URL}" \
   "--orchestrator-url=${ORCHESTRATOR_URL}" \
   "--validator-url=${VALIDATOR_URL}" \
   "--poll-frequency-secs=${POLL_FREQUENCY_SECS}" \
   "--quicknode-api-key=${QUICKNODE_API_KEY}" \
   "--private-key=${SOLVER_PRIVATE_KEY}" \
   "--solana-private-key=${SOLANA_PRIVATE_KEY}" \
   "--default-solana-cluster=${DEFAULT_SOLANA_CLUSTER}" \
   "--paid-claiming-enabled=${PAID_CLAIMING_ENABLED}" \
   "--use-bonding-curve=${USE_BONDING_CURVE}" \
   "--curve-slope=${CURVE_SLOPE}" \
   "--base-price=${BASE_PRICE}" \
   "--max-price=${MAX_PRICE}" \
   "--flat-price-per-token=${FLAT_PRICE_PER_TOKEN}"

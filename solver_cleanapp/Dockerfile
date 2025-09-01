FROM rust:1.83 AS builder

WORKDIR /usr/src/solver-cleanapp
# Install protoc for tonic-build
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

ENV CARGO_HOME=/usr/local/cargo
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

# Copy manifests and sources from local context
COPY Cargo.toml ./Cargo.toml
COPY Cargo.lock ./Cargo.lock
COPY build.rs ./build.rs
COPY src ./src
# Copy shared proto into parent dir as expected by build.rs
COPY proto ./../proto

# Build and install
RUN cargo install --locked --path .

FROM --platform=linux/amd64 debian:12-slim

RUN echo 'APT::Install-Suggests "0";' >> /etc/apt/apt.conf.d/00-docker
RUN echo 'APT::Install-Recommends "0";' >> /etc/apt/apt.conf.d/00-docker
RUN DEBIAN_FRONTEND=noninteractive \
    apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/solver-cleanapp /usr/local/bin/solver-cleanapp

CMD \
  "solver-cleanapp" \
  "--request-registrator-url=${REQUEST_REGISTRATOR_URL}" \
  "--orchestrator-url=${ORCHESTRATOR_URL}" \
  "--poll-frequency=${POLL_FREQUENCY}" \
  "--evm-private-key-hex=${EVM_PRIVATE_KEY_HEX}" \
  "--erc20-token-address=${ERC20_TOKEN_ADDRESS}" \
  "--amount-wei=${AMOUNT_WEI}" \
  "--eip155-chain-ref=${EIP155_CHAIN_REF}" 
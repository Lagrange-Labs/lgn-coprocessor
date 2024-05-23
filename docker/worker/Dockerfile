# syntax=docker/dockerfile:1.3-labs

# Stage 1: Builder
FROM rustlang/rust:nightly-slim as builder

ARG BUILD_FLAGS="--release"
ARG GO_VERSION=1.22.2
ARG INSTRUCTION_SET=native

# Install necessary packages and Go
RUN apt-get update && apt-get install -y \
    openssh-client \
    wget \
    libssl-dev \
    git \
    pkg-config \
    libssl3

# Install Go
RUN wget https://golang.org/dl/go${GO_VERSION}.linux-amd64.tar.gz && \
    tar -C /usr/local -xzf go${GO_VERSION}.linux-amd64.tar.gz && \
    rm go${GO_VERSION}.linux-amd64.tar.gz

ENV PATH="/usr/local/go/bin:${PATH}"

# Add SSH key and known hosts setup
RUN mkdir -p ~/.ssh && \
    ssh-keyscan github.com >> ~/.ssh/known_hosts && \
    chmod 600 ~/.ssh/known_hosts

# Set work directory
WORKDIR /lgn-distributed-query

# Copy the source code
COPY . .

# Build with caching
RUN --mount=type=ssh,id=default \
    --mount=type=cache,id=lgn-worker,target=/usr/local/cargo/registry \
    --mount=type=cache,id=lgn-worker,target=/usr/local/cargo/git \
    RUSTFLAGS=-Ctarget-cpu=${INSTRUCTION_SET} cargo build --bin lgn-worker --bin avs ${BUILD_FLAGS}

# Disabling tools for now
# WORKDIR /distributed-query/tools
# RUN cargo build ${BUILD_FLAGS} --config net.git-fetch-with-cli=true

# Adjust target directory if not in release mode
RUN if ! echo "$BUILD_FLAGS" | grep -q -- "--release"; then \
        mv ./target/debug ./target/release/; \
    fi

# Stage 2: Final slim runtime image for worker
FROM docker.io/library/ubuntu:22.04

# Install necessary runtime packages
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get upgrade -y && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y libssl3

RUN apt-get install -y ca-certificates

RUN update-ca-certificates

# Copy the built binary from the builder stage
COPY --from=builder /distributed-query/target/release/lgn-worker /usr/local/bin
COPY --from=builder /distributed-query/target/release/avs /usr/local/bin

# Set environment variables
ENV RUST_BACKTRACE=full
ENV CONFIG_FILE="worker.toml"
ENV RUST_MIN_STACK=1048576000

EXPOSE 8080

CMD "/usr/local/bin/lgn-worker" "--config" "$CONFIG_FILE"

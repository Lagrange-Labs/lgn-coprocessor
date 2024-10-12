# syntax=docker/dockerfile:1.3-labs

# Stage 1: Build the binary with caching
FROM base as builder

ARG BUILD_FLAGS="--release"

# Build with caching
RUN --mount=type=cache,id=lgn-worker,target=/usr/local/cargo/registry \
    --mount=type=cache,id=lgn-worker,target=/usr/local/cargo/git \
    RUSTFLAGS=-Ctarget-cpu=${INSTRUCTION_SET} cargo build --bin lgn-worker --bin lgn-avs ${BUILD_FLAGS}


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
COPY --from=builder /lgn-coprocessor/target/release/lgn-worker /usr/local/bin/worker
COPY --from=builder /lgn-coprocessor/target/release/lgn-avs /usr/local/bin/avs

# Set environment variables
ENV RUST_BACKTRACE=full
ENV CONFIG_FILE="worker.toml"
ENV RUST_MIN_STACK=1048576000

EXPOSE 8080

CMD "/usr/local/bin/worker" "--config" "$CONFIG_FILE"

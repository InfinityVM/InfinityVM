FROM rust:1.79 AS chef
WORKDIR /app

# RUN apk add --no-cache clang musl-dev gcc g++ make libc-dev git python3 py3-pip curl
RUN apt-get update && apt-get install -y --no-install-recommends clang curl libssl-dev pkg-config cmake build-essential protobuf-compiler ca-certificates libclang-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

RUN rustup toolchain install nightly

# Risc0
ENV RUSTFLAGS="-C link-arg=-lgcc"
# ENV RUSTUP_TARGETS="aarch64-unknown-linux-musl"
RUN cargo install cargo-binstall
RUN cargo binstall -y --force cargo-risczero
# RUN apk add --no-cache cmake ninja
# ENV OPENSSL_DIR "/usr/aarch64-alpine-linux-musl"
# ENV OPENSSL_INCLUDE_DIR "/usr/aarch64-alpine-linux-musl/lib"
# ENV OPENSSL_LIB_DIR "/usr/aarch64-alpine-linux-musl/lib"
# RUN ls -al /usr/lib
# RUN ls -al /usr/aarch64-alpine-linux-musl
RUN apt-get update && apt-get install -y --no-install-recommends ninja-build && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
RUN cargo risczero build-toolchain

# Prepare Cargo Chef
FROM chef as planner
COPY rust /app/rust
COPY proto /app/proto
RUN cargo chef prepare --recipe-path recipe.json

# Build dependencies
FROM chef as cacher
COPY --from=planner /app/recipe.json recipe.json
ENV LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu
RUN cargo chef cook --release --recipe-path recipe.json

# Build the application
FROM chef AS builder
ENV LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu
COPY --from=cacher /app/target /app/target
# RUN cargo build --release --target x86_64-unknown-linux-musl --locked
RUN cargo build --release --locked --manifest-path app/rust/Cargo.toml

# Create the final image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/executor /usr/local/bin/executor
CMD ["executor", "version"]



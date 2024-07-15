FROM ubuntu:20.04@sha256:3246518d9735254519e1b2ff35f95686e4a5011c90c85344c1f38df7bae9dd37 as chef
WORKDIR /app
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends clang curl libssl-dev pkg-config cmake build-essential protobuf-compiler ca-certificates libclang-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Rust
RUN curl --proto '=https' --tlsv1.2 --retry 10 --retry-connrefused -fsSL 'https://sh.rustup.rs' | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup toolchain install nightly

# Risc0
RUN cargo install cargo-binstall
RUN cargo binstall -y --force cargo-risczero
RUN cargo risczero install



# # Prepare Cargo Chef
# FROM chef as planner
# COPY . .
# RUN cargo chef prepare --recipe-path recipe.json

# # Build dependencies
# FROM chef as cacher
# RUN apt-get update && apt-get install g++-aarch64-linux-gnu musl-tools
# COPY --from=planner /app/recipe.json recipe.json
# RUN cargo chef cook --release --recipe-path recipe.json

# Build the application
FROM chef AS builder
COPY . .
ENV LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu
RUN cargo build --release --locked
# COPY --from=cacher /app/target /app/target
# RUN cargo build --release --target x86_64-unknown-linux-musl --locked

# # Create the final image
# FROM alpine:3.20
# RUN apk add --no-cache ca-certificates
# # Set the LIBCLANG_PATH environment variable
# ENV LIBCLANG_PATH=/usr/lib
# COPY --from=builder target/release/executor /usr/local/bin/executor
# CMD ["executor"]

# TODO:  in runtime image


# Start with the official Rust image to build the application
FROM rust:1.79-alpine3.20 as chef
COPY . .

# Install necessary development tools
RUN apk add --no-cache \
    clang clang-dev clang-static \
    llvm-static llvm-dev \
    build-base \
    cmake \
    linux-headers \
    openssl-dev \
    gcc \
    g++ \
    musl-dev\
    protoc \
    libgcc 

RUN rustup target add x86_64-unknown-linux-musl

# Install cargo-chef
RUN cargo install cargo-chef

# Prepare Cargo Chef
FROM chef as planner
# Set the LIBCLANG_PATH environment variable
ENV LIBCLANG_PATH=/usr/lib
RUN cargo chef prepare --recipe-path recipe.json

# Build dependencies
FROM chef as cacher
COPY --from=planner recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Build the application
FROM chef as builder
COPY . .
COPY --from=cacher /app/target /app/target
RUN cargo build --release --target x86_64-unknown-linux-musl

# Create the final image
FROM alpine:3.20
RUN apk add --no-cache ca-certificates
# Set the LIBCLANG_PATH environment variable
ENV LIBCLANG_PATH=/usr/lib
COPY --from=builder target/release/executor /usr/local/bin/executor
CMD ["executor"]
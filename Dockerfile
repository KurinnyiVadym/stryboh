# Start with the official Rust image
FROM rust:latest as builder

# Add ARMv7 target architecture for Rust
RUN rustup target add armv7-unknown-linux-gnueabihf

# Enable `armhf` architecture and install dependencies
RUN dpkg --add-architecture armhf && \
    apt-get update && \
    apt-get install -y crossbuild-essential-armhf pkg-config libssl-dev:armhf


# Create a new binary project
RUN USER=root cargo new --bin rust_arm_app
WORKDIR /rust_arm_app

# Copy your source code
COPY ./src ./src
COPY ./Cargo.toml ./Cargo.toml

# Configure pkg-config for cross-compilation
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabihf/pkgconfig

# Set OpenSSL environment variables for cross-compilation
ENV OPENSSL_DIR=/usr/include/arm-linux-gnueabihf
ENV OPENSSL_LIB_DIR=/usr/lib/arm-linux-gnueabihf
ENV OPENSSL_INCLUDE_DIR=/usr/include/arm-linux-gnueabihf

# Build your project for ARMv7
RUN cargo build --release --target=armv7-unknown-linux-gnueabihf

# Use a minimal runtime image that supports ARMv7
FROM arm32v7/debian:buster-slim
COPY --from=builder /rust_arm_app/target/armv7-unknown-linux-gnueabihf/release/rust_arm_app .

# Command to run your binary
CMD ["./rust_arm_app"]

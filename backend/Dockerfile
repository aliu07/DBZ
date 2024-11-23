# DBZ/backend/Dockerfile

# Stage 1: Build the application
FROM rust:1.74 AS builder

# Set the working directory
WORKDIR /usr/src/app

# Create a new empty shell project
RUN cargo new --bin backend
WORKDIR /usr/src/app/backend

# Copy over manifests
COPY Cargo.lock ./Cargo.lock
COPY Cargo.toml ./Cargo.toml

# Build only the dependencies to cache them
RUN cargo build --release
RUN rm src/*.rs

# Copy the source code
COPY ./src ./src

# Build for release, touching the main binary to ensure it gets rebuilt
RUN rm ./target/release/deps/backend*
RUN cargo build --release

# Stage 2: Create the runtime image
FROM debian:bookworm-slim

# Install necessary dependencies
RUN apt-get update && \
  apt-get install -y --no-install-recommends \
  ca-certificates \
  libssl-dev \
  && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

RUN mkdir -p /app/credentials

# Copy the binary from builder
COPY --from=builder /usr/src/app/backend/target/release/backend .

# Expose the port
EXPOSE 8000

# Run the binary
CMD ["./backend"]

# backend/Dockerfile

# Stage 1: Build the application
FROM rust:1.74 AS builder

# Set the working directory inside the container
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Copy the actual source code
COPY . .

# Build the application
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

# Copy the binary from builder
COPY --from=builder /app/target/release/backend .

# Expose the port
EXPOSE 8000

# Run the binary
CMD ["./backend"]

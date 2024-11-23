# backend/Dockerfile

# Stage 1: Build the application
FROM rust:latest AS builder

# Set the working directory inside the container
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Download dependencies
RUN cargo fetch

# Copy the source code
COPY src ./src

# Build the application in release mode
RUN cargo build --release

# Stage 2: Create a minimal image
FROM debian:buster-slim

# Install necessary dependencies
RUN apt-get update && apt-get install -y libssl-dev && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/backend .

# Expose the application port
EXPOSE 8000

# Define the command to run the application
CMD ["./backend"]

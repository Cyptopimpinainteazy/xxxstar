# Sidecar Dockerfile
FROM rust:latest as builder

WORKDIR /build

# Copy the entire project
COPY . .

# Build the sidecar
RUN cargo build --release -p x3-sidecar

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app directory and config directory
RUN mkdir -p /app /etc/x3

# Copy binary from builder
COPY --from=builder /build/target/release/x3-sidecar /usr/local/bin/x3-sidecar

WORKDIR /app

# Expose ports
EXPOSE 9955 9956

# Health check
HEALTHCHECK --interval=2s --timeout=5s --retries=10 \
    CMD curl -f http://localhost:9956/metrics || exit 1

# Run the sidecar with config file
CMD ["x3-sidecar", "-c", "/etc/x3/sidecar.toml"]

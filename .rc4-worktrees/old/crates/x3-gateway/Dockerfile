# Gateway Dockerfile
FROM rust:latest as builder

WORKDIR /build

# Copy the entire project
COPY . .

# Build the gateway
RUN cargo build --release -p x3-gateway

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    postgresql-client \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/x3-gateway /usr/local/bin/x3-gateway

# Copy migrations
COPY crates/x3-gateway/migrations /app/migrations

WORKDIR /app

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=2s --timeout=5s --retries=10 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the gateway
CMD ["x3-gateway"]

# Build stage
FROM rust:1.75 as builder

WORKDIR /build

# Copy the TPS tracker crate
COPY . .

# Build the TPS tracker
RUN cargo build --release --bin tps-tracker

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary from builder
COPY --from=builder /build/target/release/tps-tracker .

# Run the TPS tracker
CMD ["./tps-tracker"]

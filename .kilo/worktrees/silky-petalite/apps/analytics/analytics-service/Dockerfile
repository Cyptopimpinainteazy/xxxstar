# Build stage
FROM rust:1.74-alpine AS builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev

WORKDIR /app
COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

# Runtime stage
FROM alpine:3.19

RUN apk add --no-cache ca-certificates curl

WORKDIR /app

COPY --from=builder /app/target/release/analytics-service /app/analytics-service

# Create non-root user
RUN addgroup -g 1000 analytics && \
    adduser -D -s /bin/sh -u 1000 -G analytics analytics && \
    chown -R analytics:analytics /app

USER analytics

EXPOSE 8080

CMD ["/app/analytics-service"]

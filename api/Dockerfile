# Build stage  
FROM rust:1.82-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY api ./api
COPY node-agent ./node-agent

# Regenerate Cargo.lock if version incompatible
RUN cargo generate-lockfile || true

# Build the API service
RUN cargo build --release --package api

# Runtime stage
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache libgcc openssl ca-certificates curl

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/api /app/api

# Expose port
EXPOSE 8080

# Run the application
CMD ["/app/api"]

# Build stage
FROM rustlang/rust:nightly-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY api ./api
COPY node-agent ./node-agent

# Build the node-agent
RUN cargo build --release --package node-agent

# Runtime stage
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache libgcc openssl ca-certificates curl

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/node-agent /app/node-agent

# Create configuration directory
RUN mkdir -p /etc/node-agent

# Expose Xray API port (for internal communication)
EXPOSE 10085

# Run the application
CMD ["/app/node-agent"]

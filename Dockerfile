# Stage 1: Builder
FROM rust:latest AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy the entire workspace
COPY . .

# Build the api crate in release mode
RUN cargo build --release --bin api

# Stage 2: Runtime
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies and curl for healthcheck
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from the builder stage and rename it to nebripop
COPY --from=builder /usr/src/app/target/release/api ./nebripop

# Copy migrations and templates
COPY --from=builder /usr/src/app/migrations ./migrations
COPY --from=builder /usr/src/app/crates/api/templates ./crates/api/templates

# Expose the API port
EXPOSE 8080

# Healthcheck
HEALTHCHECK --interval=30s --timeout=3s \
  CMD curl -f http://localhost:8080/health || exit 1

# Start the application
CMD ["./nebripop"]

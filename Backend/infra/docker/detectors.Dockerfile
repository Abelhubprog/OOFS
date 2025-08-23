# Multi-stage Dockerfile for OOF Detectors Service
# Optimized for production deployment with minimal attack surface

# Build stage
FROM rust:1.79-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    build-essential \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user for build
RUN useradd -m -u 1001 appuser

# Set working directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./
COPY crates/detectors/Cargo.toml ./crates/detectors/
COPY crates/shared/Cargo.toml ./crates/shared/

# Create dummy source files to cache dependencies
RUN mkdir -p crates/detectors/src crates/shared/src \
    && echo "fn main() {}" > crates/detectors/src/main.rs \
    && echo "" > crates/shared/src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release --bin detectors

# Remove dummy source files
RUN rm -rf crates/*/src/

# Copy actual source code
COPY crates/ ./crates/
COPY configs/ ./configs/
COPY db/ ./db/

# Build the actual application
RUN cargo build --release --bin detectors

# Strip binary to reduce size
RUN strip target/release/detectors

# Runtime stage
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user
RUN useradd -r -u 1001 -g users -d /app -s /sbin/nologin -c "Detectors Service" appuser

# Create directories with proper permissions
RUN mkdir -p /app/configs /app/logs /tmp/detectors \
    && chown -R appuser:users /app /tmp/detectors \
    && chmod 755 /app \
    && chmod 750 /app/logs \
    && chmod 1777 /tmp/detectors

# Copy binary from builder
COPY --from=builder --chown=appuser:users /app/target/release/detectors /app/detectors

# Copy configuration files
COPY --from=builder --chown=appuser:users /app/configs/ /app/configs/

# Set working directory
WORKDIR /app

# Switch to non-root user
USER appuser

# Expose metrics port
EXPOSE 8083

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8083/health || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV DETECTORS_PORT=8083
ENV DETECTORS_HOST=0.0.0.0

# Resource limits (these can be overridden in docker-compose)
LABEL \
    org.opencontainers.image.title="OOF Detectors Service" \
    org.opencontainers.image.description="OOF moment detection engine" \
    org.opencontainers.image.vendor="OOF Team" \
    org.opencontainers.image.version="1.0.0" \
    org.opencontainers.image.created="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
    org.opencontainers.image.source="https://github.com/oof/backend" \
    org.opencontainers.image.licenses="MIT"

# Security labels
LABEL \
    security.non-root="true" \
    security.no-new-privileges="true" \
    security.read-only-root="false"

# Default command
CMD ["./detectors"]

# Alternative entrypoint for debugging
# ENTRYPOINT ["./detectors"]
# CMD ["--config", "configs/detectors.yaml", "--log-level", "info"]

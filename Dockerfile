# Build stage
FROM rust:1.89-slim@sha256:33219ca58c0dd38571fd3f87172b5bce2d9f3eb6f27e6e75efe12381836f71fa AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace configuration
COPY Cargo.toml Cargo.lock ./

# Copy all crates
COPY filter-core/ ./filter-core/
COPY filter-web/ ./filter-web/
COPY miniflux-filter/ ./miniflux-filter/

# Build release binary
RUN cargo build --release --bin miniflux-filter

# Runtime stage
FROM debian:bookworm-slim@sha256:2424c1850714a4d94666ec928e24d86de958646737b1d113f5b2207be44d37d8

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false -m -d /app appuser

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/miniflux-filter /usr/local/bin/miniflux-filter

# Copy static web assets
COPY --from=builder /app/filter-web/static/ ./static/

# Create rules directory
RUN mkdir -p ./rules && chown -R appuser:appuser /app

# Switch to app user
USER appuser

# Expose default web port
EXPOSE 8080

# Set default environment variables
ENV MINIFLUX_FILTER_WEB_ENABLED=true
ENV MINIFLUX_FILTER_WEB_PORT=8080
ENV MINIFLUX_FILTER_POLL_INTERVAL=300
ENV MINIFLUX_FILTER_RULES_DIR=/app/rules

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD timeout 5 bash -c '</dev/tcp/localhost/8080' || exit 1

# Run the application
CMD ["miniflux-filter"]

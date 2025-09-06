# Build stage
FROM rust:1.89-slim@sha256:e556a015ecb064ca6b3b74bceb36a54deaf88afbe2956b8fe3e445da446d9cf8 AS builder

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
FROM debian:bookworm-slim@sha256:b1a741487078b369e78119849663d7f1a5341ef2768798f7b7406c4240f86aef

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

# Multi-stage build for Zenoh Recorder
FROM rust:1.75 as builder

WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY proto/ ./proto/

# Copy source code
COPY src/ ./src/

# Build release binary
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 zenoh && \
    mkdir -p /data && \
    chown -R zenoh:zenoh /data

USER zenoh
WORKDIR /home/zenoh

# Copy binary from builder
COPY --from=builder /build/target/release/zenoh-recorder /usr/local/bin/

# Environment variables with defaults
ENV DEVICE_ID=recorder-001
ENV REDUCTSTORE_URL=http://reductstore:8383
ENV BUCKET_NAME=zenoh_data
ENV RUST_LOG=zenoh_recorder=info

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD pgrep zenoh-recorder || exit 1

# Run the recorder
CMD ["zenoh-recorder"]


# Multi-stage build for bwb_ai
# Stage 1: Builder
FROM rust:latest as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy source
COPY . .

# Build release binary
RUN cargo build --release --quiet

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 ai && \
    mkdir -p /home/ai/.ollama && \
    mkdir -p /home/ai/.bwb_ai && \
    chown -R ai:ai /home/ai

# Copy binary from builder
COPY --from=builder /app/target/release/bwb_ai /usr/local/bin/bwb_ai

# Set user
USER ai

# Environment
ENV RUST_LOG=info
ENV PATH="/usr/local/bin:${PATH}"

# Volumes
VOLUME ["/home/ai/.ollama"]
VOLUME ["/home/ai/.bwb_ai"]

# WebSocket server default
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080 || exit 1

# Default command: WebSocket server
# Can be overridden: docker run bwb_ai:latest --tui
ENTRYPOINT ["bwb_ai"]
CMD ["--ws", "--ws_addr", "0.0.0.0:8080"]

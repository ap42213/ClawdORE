FROM rust:1.85-slim-bookworm as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    gcc \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy source code
COPY . .

# Build release
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries from builder
COPY --from=builder /app/target/release/simulation-bot /app/
COPY --from=builder /app/target/release/monitor-bot /app/
COPY --from=builder /app/target/release/miner-bot /app/
COPY --from=builder /app/target/release/betting-bot /app/
COPY --from=builder /app/target/release/analytics-bot /app/

# Default to simulation bot
CMD ["./simulation-bot"]

FROM rust:1.75 as builder

WORKDIR /app

# Copy the entire clawdbot project
COPY clawdbot/ ./clawdbot/
COPY ore/ ./ore/

# Build the bots
WORKDIR /app/clawdbot
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy built binaries
COPY --from=builder /app/clawdbot/target/release/monitor-bot /usr/local/bin/
COPY --from=builder /app/clawdbot/target/release/analytics-bot /usr/local/bin/
COPY --from=builder /app/clawdbot/target/release/miner-bot /usr/local/bin/
COPY --from=builder /app/clawdbot/target/release/betting-bot /usr/local/bin/

# Copy config example
COPY clawdbot/config.example.json /app/config.example.json

WORKDIR /app

# Default to monitor bot (safest)
CMD ["monitor-bot"]

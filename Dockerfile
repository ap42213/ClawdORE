FROM rust:1.85-slim-bookworm as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    gcc \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the entire clawdbot project
COPY clawdbot/ ./clawdbot/
COPY ore/ ./ore/

# Build the bots WITH DATABASE FEATURE
WORKDIR /app/clawdbot
RUN cargo build --release --features database

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy ALL built binaries
COPY --from=builder /app/clawdbot/target/release/coordinator-bot /app/
COPY --from=builder /app/clawdbot/target/release/monitor-bot /app/
COPY --from=builder /app/clawdbot/target/release/parser-bot /app/
COPY --from=builder /app/clawdbot/target/release/miner-bot /app/
COPY --from=builder /app/clawdbot/target/release/betting-bot /app/
COPY --from=builder /app/clawdbot/target/release/analytics-bot /app/
COPY --from=builder /app/clawdbot/target/release/learning-bot /app/

# Create startup script that selects bot based on BOT_TYPE env var
RUN echo '#!/bin/sh\n\
BOT=${BOT_TYPE:-coordinator-bot}\n\
echo "Starting $BOT..."\n\
echo "Available bots: coordinator-bot, miner-bot, learning-bot, monitor-bot, parser-bot, analytics-bot, betting-bot"\n\
exec /app/$BOT\n\
' > /app/start.sh && chmod +x /app/start.sh

# Default to coordinator bot via start script
CMD ["/app/start.sh"]

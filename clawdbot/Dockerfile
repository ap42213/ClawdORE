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
COPY --from=builder /app/target/release/monitor-bot /app/
COPY --from=builder /app/target/release/parser-bot /app/
COPY --from=builder /app/target/release/miner-bot /app/
COPY --from=builder /app/target/release/betting-bot /app/
COPY --from=builder /app/target/release/analytics-bot /app/

# Create startup script that selects bot based on BOT_TYPE env var
RUN echo '#!/bin/sh\n\
BOT=${BOT_TYPE:-parser-bot}\n\
echo "Starting $BOT..."\n\
exec /app/$BOT\n\
' > /app/start.sh && chmod +x /app/start.sh

# Default to parser bot via start script
CMD ["/app/start.sh"]

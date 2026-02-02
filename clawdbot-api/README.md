# ClawdBot API Server

REST API backend for controlling ClawdBot instances on Railway.

## Endpoints

### Health Check
```bash
GET /health
```

### List Bots
```bash
GET /api/bots
```

Response:
```json
{
  "bots": [
    {
      "id": "monitor",
      "name": "Monitor Bot",
      "status": "running",
      "uptime": 3600
    }
  ]
}
```

### Start Bot
```bash
POST /api/bots/:id/start
```

### Stop Bot
```bash
POST /api/bots/:id/stop
```

### Get Bot Status
```bash
GET /api/bots/:id/status
```

### Get Bot Logs
```bash
GET /api/bots/:id/logs
```

## Run Locally

```bash
cd clawdbot-api
cargo run
```

## Deploy to Railway

Railway automatically detects Rust and builds the API:

```bash
railway up
```

## Environment Variables

```env
RUST_LOG=info
PORT=3000
SOLANA_KEYPAIR=your-base64-keypair
RPC_URL=https://api.mainnet-beta.solana.com
```

## Bot Management

The API manages bot processes:
- Spawns bot binaries as child processes
- Monitors process health
- Captures logs
- Handles graceful shutdown

## CORS

Configured to allow Vercel frontend:

```rust
.layer(
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
)
```

## Production Notes

- Use environment variables for secrets
- Enable HTTPS only in production
- Rate limit API endpoints
- Add authentication for security

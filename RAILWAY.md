# Railway Deployment Guide

## Deploy ClawdBot to Railway

### Prerequisites
- Railway account (free tier works!)
- GitHub repository pushed

### Quick Deploy

1. **Install Railway CLI**
```bash
npm i -g @railway/cli
```

2. **Login to Railway**
```bash
railway login
```

3. **Initialize Project**
```bash
railway init
```

4. **Set Environment Variables**
```bash
# Set your Solana private key
railway variables set SOLANA_KEYPAIR="your-base64-encoded-keypair"

# Set RPC URL
railway variables set RPC_URL="https://api.mainnet-beta.solana.com"

# Or use devnet for testing
railway variables set RPC_URL="https://api.devnet.solana.com"
```

5. **Deploy!**
```bash
railway up
```

### Deploy Multiple Bots

Railway lets you run multiple services. Create separate services for each bot:

#### Service 1: Monitor Bot
```bash
railway service create monitor-bot
railway up --service monitor-bot
# Set start command: cd clawdbot && ./target/release/monitor-bot
```

#### Service 2: Analytics Bot
```bash
railway service create analytics-bot
railway up --service analytics-bot
# Set start command: cd clawdbot && ./target/release/analytics-bot
```

#### Service 3: Miner Bot (Careful - uses SOL!)
```bash
railway service create miner-bot
railway up --service miner-bot
# Set start command: cd clawdbot && ./target/release/miner-bot
```

### Configure in Railway Dashboard

1. Go to railway.app
2. Select your project
3. Click on each service
4. Go to "Settings" → "Environment"
5. Add variables:
   - `RUST_LOG=info`
   - `SOLANA_KEYPAIR=<your-keypair>`
   - `RPC_URL=<your-rpc-url>`

### Monitoring

Railway provides:
- Real-time logs
- CPU/Memory metrics
- Automatic restarts
- Custom domains

### Costs

- Free tier: $5/month credit
- Pro: $20/month
- Each bot uses ~100-200 MB RAM
- Minimal CPU usage

### Tips

1. **Start with Monitor Bot** (safest, no spending)
2. **Use devnet first** to test
3. **Set up alerts** in Railway dashboard
4. **Monitor logs** regularly
5. **Scale as needed**

### Rollback

```bash
railway rollback
```

### Check Status

```bash
railway status
```

### View Logs

```bash
railway logs
```

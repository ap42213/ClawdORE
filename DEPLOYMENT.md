# Deploy ClawdBot to Railway + Vercel

Complete guide for deploying your bot system to production.

## Architecture

```
┌─────────────────┐
│  Vercel         │
│  (Frontend)     │ ← User Interface
│  - Dashboard    │
│  - Bot Controls │
└────────┬────────┘
         │ HTTPS
         ▼
┌─────────────────┐
│  Railway        │
│  (Backend)      │ ← API Server
│  - REST API     │
│  - Bot Manager  │
└────────┬────────┘
         │
    ┌────┴────┬────────┬────────┐
    ▼         ▼        ▼        ▼
┌────────┐┌────────┐┌────────┐┌────────┐
│Monitor ││Analytics││ Miner  ││Betting │
│  Bot   ││  Bot   ││  Bot   ││  Bot   │
└────────┘└────────┘└────────┘└────────┘
```

## Step 1: Deploy API to Railway

### Install Railway CLI
```bash
npm i -g @railway/cli
railway login
```

### Create Project
```bash
cd /workspaces/ClawdORE
railway init
```

### Set Environment Variables
```bash
railway variables set RUST_LOG=info
railway variables set PORT=3000
railway variables set SOLANA_KEYPAIR="your-base64-keypair"
railway variables set RPC_URL="https://api.mainnet-beta.solana.com"
```

### Deploy API
```bash
cd clawdbot-api
railway up
```

### Get API URL
```bash
railway status
# Copy the domain (e.g., clawdbot-api.railway.app)
```

## Step 2: Deploy Bots to Railway

Railway supports multiple services in one project.

### Service: Monitor Bot
```bash
railway service create monitor-bot
railway up --service monitor-bot --dir clawdbot
# In Railway dashboard, set start command:
# cd clawdbot && cargo run --release --bin monitor-bot
```

### Service: Analytics Bot
```bash
railway service create analytics-bot
railway up --service analytics-bot --dir clawdbot
# Set start command:
# cd clawdbot && cargo run --release --bin analytics-bot
```

### Service: Miner Bot (Optional - costs SOL!)
```bash
railway service create miner-bot
railway up --service miner-bot --dir clawdbot
# Set start command:
# cd clawdbot && cargo run --release --bin miner-bot
```

## Step 3: Deploy Frontend to Vercel

### Install Vercel CLI
```bash
npm i -g vercel
vercel login
```

### Deploy
```bash
cd frontend
npm install
vercel
```

### Set Environment Variables
In Vercel dashboard or CLI:
```bash
vercel env add NEXT_PUBLIC_API_URL
# Enter: https://your-railway-api-url.railway.app
```

### Redeploy
```bash
vercel --prod
```

## Step 4: Configure & Test

### Test API
```bash
curl https://your-api.railway.app/health
curl https://your-api.railway.app/api/bots
```

### Test Frontend
1. Open your Vercel URL
2. Try starting Monitor Bot
3. Check Railway logs

### View Logs

Railway:
```bash
railway logs --service api
railway logs --service monitor-bot
```

Vercel:
```bash
vercel logs
```

## Environment Variables Summary

### Railway (API + Bots)
```env
RUST_LOG=info
PORT=3000
SOLANA_KEYPAIR=<base64-encoded-keypair>
RPC_URL=https://api.mainnet-beta.solana.com
```

### Vercel (Frontend)
```env
NEXT_PUBLIC_API_URL=https://your-api.railway.app
```

## Cost Estimate

### Railway
- **Hobby**: $5/month credit (free)
- **Pro**: $20/month
- Each service: ~$0.50-2/hour when active
- Start with Monitor Bot only (safest)

### Vercel
- **Hobby**: Free (100GB bandwidth, unlimited requests)
- **Pro**: $20/month (if you need more)

### Total for Testing
- Start with Railway free tier + Vercel free tier = $0

## Security Best Practices

1. **Never commit secrets**
   ```bash
   # Add to .gitignore
   echo ".env" >> .gitignore
   echo ".env.local" >> .gitignore
   ```

2. **Use environment variables**
   - Store keypairs in Railway secrets
   - Use secure RPC endpoints
   - Enable Railway's private networking

3. **Start with devnet**
   ```bash
   railway variables set RPC_URL="https://api.devnet.solana.com"
   ```

4. **Add authentication** (future)
   - Implement API keys
   - Use JWT tokens
   - Add rate limiting

## Monitoring

### Railway Dashboard
- View real-time logs
- Monitor CPU/memory usage
- Set up alerts
- Track deployments

### Vercel Analytics
- Page views
- Function executions
- Error tracking
- Performance metrics

## Troubleshooting

### Bot won't start
```bash
# Check Railway logs
railway logs --service monitor-bot

# Verify environment variables
railway variables

# Test locally first
cd clawdbot
cargo run --release --bin monitor-bot
```

### API connection failed
```bash
# Check API is running
curl https://your-api.railway.app/health

# Verify CORS settings in API code
# Check frontend API URL is correct
```

### Build fails
```bash
# Clear Railway cache
railway down
railway up --force

# Check Cargo.toml dependencies
# Verify Rust version
```

## Scaling

### When to scale:
- Monitor bot: Always on (low cost)
- Analytics bot: Run hourly/daily
- Miner bot: Only when profitable
- Betting bot: Only during active rounds

### Auto-scaling with Railway:
```bash
# Set replica count
railway service update --replicas 2
```

## Rollback

### Railway
```bash
railway rollback
```

### Vercel
```bash
vercel rollback
```

## Next Steps

1. ✅ Deploy API to Railway
2. ✅ Deploy Monitor Bot
3. ✅ Deploy Frontend to Vercel
4. ⏸️ Test Monitor Bot (safe)
5. ⏸️ Deploy Analytics Bot
6. ⏸️ Test on devnet first
7. ⏸️ Deploy Miner/Betting (production)

## Support

- Railway Docs: https://docs.railway.app
- Vercel Docs: https://vercel.com/docs
- Discord: Create your own support channel

## Emergency Stop

```bash
# Stop all Railway services
railway down

# Or in Railway dashboard:
# Click service → Settings → Stop
```

---

**Ready to deploy?** Start with Step 1! 🚀

# Railway Deployment for Simulation Bot

## Quick Deploy

### 1. Install Railway CLI
```bash
npm i -g @railway/cli
railway login
```

### 2. Create Wallet (No SOL Needed!)
```bash
cd /workspaces/ClawdORE/clawdbot
solana-keygen new -o wallet.json --no-bip39-passphrase
```

### 3. Initialize Railway Project
```bash
railway init
# Select: Create new project
# Name it: clawdbot-simulation
```

### 4. Set Environment Variables
```bash
railway variables set RPC_URL="https://api.mainnet-beta.solana.com"
railway variables set RUST_LOG="info"
```

### 5. Upload Wallet
```bash
# Copy wallet contents
cat wallet.json

# In Railway dashboard:
# Settings → Variables → Add Variable
# Name: SOLANA_KEYPAIR
# Value: (paste entire wallet.json contents as JSON)
```

### 6. Deploy!
```bash
railway up
```

## Monitor Your Simulation

### View Logs
```bash
railway logs
```

You'll see:
```
🎮 ORE Simulation Bot Starting...
📡 Connecting to mainnet
💰 Starting with 10 SOL (simulated)
📊 Round 12345 - Split { participants: 42 }
✅ Earned 0.0238 ORE
```

### Check Status
```bash
railway status
```

### Download Results
Results are exported to `simulation_results.json` which you can retrieve via:
```bash
railway run cat simulation_results.json > local_results.json
```

## Cost Estimate

- **Free Tier**: $5/month credit
- **Simulation Bot**: ~$0.50-1/month
- **Runs 24/7**

## Alternatives to Railway

### Option 1: DigitalOcean Droplet
$6/month - Basic droplet, SSH in and run

### Option 2: AWS EC2 Free Tier
Free for 1 year - t2.micro instance

### Option 3: Your Local Machine (if you have one)
```bash
# On Mac/Linux/Windows
cargo run --release --bin simulation-bot
# Keep computer running
```

### Option 4: Fly.io
Similar to Railway, also has free tier

## Best Option: Railway

**Why Railway:**
- ✅ Easy deployment
- ✅ Automatic restarts
- ✅ View logs in dashboard
- ✅ Free tier available
- ✅ No server management

## What Runs on Railway

Your simulation bot will:
- Monitor mainnet ORE 24/7
- Track every 60-second round
- Log all splits/full ore/motherlode
- Test your strategies
- Collect 1000+ rounds of data

## When to Check In

- **Daily**: Quick log check
- **Weekly**: Download results, analyze
- **Monthly**: Compare strategies

## Deployment Troubleshooting

### "Build Failed"
```bash
# Check Railway logs
railway logs

# Common fix: Add rust-toolchain file
echo "1.70.0" > rust-toolchain
railway up
```

### "Out of Memory"
Railway free tier has 512MB RAM. If bot crashes:
1. Reduce history_depth in config
2. Disable database features
3. Upgrade Railway plan ($5/month for 1GB)

### "Connection Errors"
Try different RPC:
```bash
railway variables set RPC_URL="https://solana-api.projectserum.com"
# or
railway variables set RPC_URL="https://api.devnet.solana.com"
```

## Success Criteria

After 1 week on Railway, you should have:
- ✅ 10,000+ rounds tracked
- ✅ Clear split vs full ORE patterns
- ✅ Motherlode frequency data
- ✅ Strategy performance metrics

Then you can decide whether to deploy live trading bots!

---

**Ready to deploy?** Run `railway init` in `/workspaces/ClawdORE/clawdbot`!

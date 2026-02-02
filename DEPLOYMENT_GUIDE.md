# 🚀 Running ClawdBot - All Options

## Your Environment: GitHub Codespaces ✅

Good news! Codespaces is perfect for running ClawdBot. Here are all your options:

---

## 📌 **Option 1: Direct in Codespaces (RECOMMENDED)**

### Super Quick Start:

```bash
# Run the setup script
./setup-codespaces.sh

# Configure
cd clawdbot
cp config.example.json config.json
nano config.json  # Edit your settings

# Run a bot
RUST_LOG=info ./target/release/monitor-bot
```

### Why This Works:
✅ Codespaces has Docker pre-installed  
✅ You have full Linux environment  
✅ Can run Rust, Python, Node.js, anything  
✅ 60 hours free per month  
✅ Ports are auto-forwarded to your browser  

### To Keep It Running:
- Pin your Codespace
- Use `tmux` to keep sessions alive
- Enable auto-suspend settings

---

## 🐳 **Option 2: Docker (Portable)**

### Build and Run:

```bash
# Build the Docker image
docker build -t clawdbot .

# Run monitor bot
docker run -v $(pwd)/clawdbot/config.json:/app/config.json clawdbot monitor-bot

# Or use docker-compose for all bots
docker-compose up
```

### Why Docker:
✅ Works on ANY system  
✅ Isolated environment  
✅ Easy to deploy to cloud  
✅ No Rust installation needed  

### Deploy Docker Anywhere:
- DigitalOcean App Platform
- AWS ECS
- Google Cloud Run
- Azure Container Instances
- Railway.app
- Fly.io

---

## ☁️ **Option 3: GitHub Actions (Automated)**

### Setup:

1. Push your code to GitHub
2. Go to Actions tab
3. Enable workflows
4. Click "Run workflow" to start

### Configuration:

Store your wallet key as a GitHub Secret:
1. Go to Settings → Secrets → Actions
2. Add `SOLANA_PRIVATE_KEY`
3. Bots will run on schedule

### Why GitHub Actions:
✅ Completely free (2000 minutes/month)  
✅ Runs in cloud  
✅ Automated schedules  
✅ No server needed  
✅ Download results as artifacts  

---

## 🌐 **Option 4: Run Web Terminal in Codespaces**

```bash
cd clawdbot-web
cargo run --release
```

Then in Codespaces:
1. Click the "Ports" tab
2. Find port 3000
3. Click "Open in Browser"
4. Control bots from the web UI!

### Why Web Terminal:
✅ Beautiful interface  
✅ No command line needed  
✅ Works in Codespaces perfectly  
✅ Control multiple bots  

---

## 💻 **Option 5: Cloud Platforms**

### Replit
1. Import your GitHub repo
2. Click "Run"
3. That's it!

### Railway.app
```bash
# Install Railway CLI
npm install -g @railway/cli

# Deploy
railway init
railway up
```

### Fly.io
```bash
# Install flyctl
curl -L https://fly.io/install.sh | sh

# Deploy
flyctl launch
flyctl deploy
```

---

## 🎯 **Quick Decision Guide**

### Just Want to Try It?
→ **Option 1: Direct in Codespaces**

### Want Easy Deployment?
→ **Option 2: Docker**

### Want It Automated?
→ **Option 3: GitHub Actions**

### Want a Pretty Interface?
→ **Option 4: Web Terminal**

### Want Professional Hosting?
→ **Option 5: Cloud Platforms**

---

## 🚀 **Getting Started RIGHT NOW in Codespaces**

### Step 1: Setup (one time)
```bash
./setup-codespaces.sh
```

### Step 2: Configure
```bash
cd clawdbot
cp config.example.json config.json

# For testing, use devnet (free SOL)
nano config.json
# Set: "rpc_url": "https://api.devnet.solana.com"
```

### Step 3: Create Test Wallet
```bash
# Install Solana CLI (if not done)
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Generate wallet
solana-keygen new --outfile ~/.config/solana/id.json

# Get free devnet SOL
solana config set --url devnet
solana airdrop 2
```

### Step 4: Run!
```bash
# Start with the safe monitor bot
RUST_LOG=info ./target/release/monitor-bot
```

---

## 🎮 **Interactive Mode (Easiest)**

```bash
cd clawdbot
./run.sh
```

Choose from the menu:
1. Monitor Bot (safe to start)
2. Analytics Bot (safe)
3. Miner Bot (uses SOL)
4. Betting Bot (uses SOL)
5. Run All in tmux

---

## 📊 **Running in Background**

### Using tmux:
```bash
# Start tmux session
tmux new -s clawdbot

# Run bot
RUST_LOG=info ./target/release/monitor-bot

# Detach: Ctrl+B then D
# Reattach: tmux attach -t clawdbot
```

### Using screen:
```bash
screen -S clawdbot
RUST_LOG=info ./target/release/monitor-bot
# Detach: Ctrl+A then D
# Reattach: screen -r clawdbot
```

### Using nohup:
```bash
nohup ./target/release/monitor-bot > bot.log 2>&1 &
```

---

## 🔒 **Security Tips**

1. **Never commit your wallet keys**
2. **Use .gitignore** (already set up)
3. **Test on devnet first**
4. **Start with small amounts**
5. **Use environment variables for secrets**

---

## 💡 **Pro Tips for Codespaces**

### Keep Your Codespace Alive:
```bash
# Install keep-alive script
while true; do echo "alive"; sleep 300; done &
```

### Save Costs:
- Set timeout to 30 minutes of inactivity
- Stop when not in use
- Use smaller machine types

### Persist Data:
- Commit config changes
- Push to GitHub regularly
- Export important data

---

## 🆘 **Troubleshooting**

### "Command not found"
```bash
source "$HOME/.cargo/env"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

### "Permission denied"
```bash
chmod +x ./run.sh
chmod +x ./target/release/*
```

### "Port already in use"
```bash
# Kill the process
lsof -ti:3000 | xargs kill -9
```

### "Out of SOL"
```bash
# On devnet
solana airdrop 2

# On mainnet - you need to fund your wallet
```

---

## 🎉 **Recommended Setup for You**

Since you're in **GitHub Codespaces**:

1. **Run the setup script**:
   ```bash
   ./setup-codespaces.sh
   ```

2. **Use devnet for testing**:
   ```bash
   solana config set --url devnet
   solana airdrop 2
   ```

3. **Start with Monitor Bot**:
   ```bash
   cd clawdbot
   RUST_LOG=info ./target/release/monitor-bot
   ```

4. **Once comfortable, try Docker**:
   ```bash
   docker-compose up
   ```

5. **For automation, use GitHub Actions** (no Codespace needed!)

---

## 📞 **Need Help?**

- Check the logs: `RUST_LOG=debug`
- Review config: `cat config.json`
- Test connection: `solana balance`
- Verify build: `cargo check`

---

**You're all set! Pick an option and start mining! ⛏️💎**

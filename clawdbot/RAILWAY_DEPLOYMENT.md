# Railway Deployment Guide for ClawdORE

## 🚂 Railway Configuration

### Build Settings (Auto-detected)
- **Builder**: Docker (using Dockerfile)
- **Start Command**: Auto-selects bot based on `BOT_TYPE` env var

### Root Directory
Set to: `clawdbot`

---

## 🤖 Available Bots

| Bot | `BOT_TYPE` Value | Description |
|-----|-----------------|-------------|
| **Parser Bot** | `parser-bot` (default) | Expert blockchain parser - monitors ALL ORE transactions |
| **Monitor Bot** | `monitor-bot` | Read-only monitoring of ORE rounds |
| **Miner Bot** | `miner-bot` | Automated mining (requires `BOT_MODE=live`) |
| **Betting Bot** | `betting-bot` | Automated betting (requires `BOT_MODE=live`) |
| **Analytics Bot** | `analytics-bot` | Collects and displays ORE statistics |

---

## 🔧 Required Environment Variables

### Core Settings (REQUIRED)

| Variable | Description | Example |
|----------|-------------|---------|
| `RPC_URL` | Solana RPC endpoint | `https://api.mainnet-beta.solana.com` |
| `KEYPAIR_B58` | Base58 encoded private key | `5K8hg...` (64 bytes as base58) |

### Alternative Keypair Methods

You can use **ONE** of these:

| Variable | Description |
|----------|-------------|
| `KEYPAIR_B58` | Base58 encoded 64-byte private key |
| `KEYPAIR_JSON` | JSON array of bytes `[1,2,3,...]` |
| `KEYPAIR_PATH` | Path to keypair file (if mounted) |

---

## 🎮 Bot Selection & Mode

| Variable | Default | Description |
|----------|---------|-------------|
| `BOT_TYPE` | `parser-bot` | Which bot to run (see table above) |
| `BOT_MODE` | `simulation` | `simulation`, `monitor`, or `live` |
| `RUST_LOG` | `info` | Log level: `debug`, `info`, `warn`, `error` |

⚠️ **Important**: Set `BOT_MODE=live` only if you want real transactions!

---

## 🔍 Parser Bot Settings (Optional)

| Variable | Default | Description |
|----------|---------|-------------|
| `PARSER_INTERVAL` | `30` | Seconds between parsing updates |
| `PARSER_TX_LIMIT` | `50` | Number of transactions to fetch per cycle |

---

## ⛏️ Mining Configuration (Optional)

| Variable | Default | Description |
|----------|---------|-------------|
| `MINING_ENABLED` | `true` | Enable mining simulation |
| `MINING_STRATEGY` | `weighted` | `random`, `weighted`, `hot_squares`, `contrarian` |
| `DEPLOY_AMOUNT_SOL` | `0.1` | SOL to deploy per round |
| `MIN_SOL_BALANCE` | `0.5` | Minimum SOL to maintain |
| `AUTO_CLAIM_THRESHOLD` | `10.0` | ORE threshold for auto-claim |
| `USE_AUTOMATION` | `true` | Use ORE automation feature |
| `MAX_AUTOMATION_BALANCE` | `1.0` | Max SOL in automation |

---

## 🎲 Betting Configuration (Optional)

| Variable | Default | Description |
|----------|---------|-------------|
| `BETTING_ENABLED` | `false` | Enable betting simulation |
| `BETTING_STRATEGY` | `spread` | `spread`, `focused`, `kelly`, `martingale` |
| `BET_PERCENTAGE` | `0.05` | % of balance to bet per round |
| `MAX_BET_SOL` | `0.5` | Maximum bet amount |
| `MIN_BET_SOL` | `0.01` | Minimum bet amount |
| `RISK_TOLERANCE` | `0.5` | 0.0 (safe) to 1.0 (aggressive) |
| `SQUARES_TO_BET` | `3` | Number of squares to bet on |

---

## 📊 Analytics Configuration (Optional)

| Variable | Default | Description |
|----------|---------|-------------|
| `ANALYTICS_ENABLED` | `true` | Enable analytics collection |
| `HISTORY_DEPTH` | `100` | Rounds of history to track |
| `UPDATE_INTERVAL` | `60` | Seconds between updates |
| `EXPORT_PATH` | `simulation_results.json` | Results export location |

---

## 🚀 Quick Setup for Railway

### Step 1: Create New Project
1. Go to [Railway.app](https://railway.app)
2. Click "New Project" → "Deploy from GitHub repo"
3. Select your ClawdORE repository

### Step 2: Configure Settings
1. Go to **Settings** → **General**
2. Set **Root Directory** to: `clawdbot`

### Step 3: Add Variables
Go to **Variables** tab and add:

```
RPC_URL=https://api.mainnet-beta.solana.com
KEYPAIR_B58=your_base58_private_key_here
BOT_TYPE=parser-bot
PARSER_INTERVAL=30
PARSER_TX_LIMIT=50
RUST_LOG=info
```

### Step 4: Deploy
Railway will automatically build and deploy!

---

## 🔄 Running Multiple Bots

To run multiple bots, create separate Railway services in the same project:

1. **Service 1: Simulation Bot**
   - `BOT_TYPE=simulation-bot`
   
2. **Service 2: Monitor Bot**  
   - `BOT_TYPE=monitor-bot`

3. **Service 3: Analytics Bot**
   - `BOT_TYPE=analytics-bot`

All services can share the same `RPC_URL` and `KEYPAIR_B58` variables.

---

## 🔑 Getting Your KEYPAIR_B58

From your Solana CLI keypair file, convert to base58:

```bash
# View your keypair as JSON array
cat ~/.config/solana/id.json

# Convert to base58 using Python
python3 -c "
import json
import base58
with open('$HOME/.config/solana/id.json') as f:
    key = json.load(f)
print(base58.b58encode(bytes(key)).decode())
"
```

Or use a Solana wallet that exports base58 private keys.

---

## 📈 Recommended RPC Providers

For better performance, use a dedicated RPC:

| Provider | Free Tier | Paid Plans |
|----------|-----------|------------|
| [Helius](https://helius.dev) | 100k req/day | From $49/mo |
| [QuickNode](https://quicknode.com) | 10M credits | From $49/mo |
| [Triton](https://triton.one) | Limited | Enterprise |
| [Alchemy](https://alchemy.com) | 300M CU/mo | From $0 |

Example with Helius:
```
RPC_URL=https://mainnet.helius-rpc.com/?api-key=YOUR_KEY
```

---

## ⚠️ Security Notes

1. **Never commit your keypair** to git
2. Use Railway's encrypted environment variables
3. For production, use a **dedicated wallet** with limited funds
4. Start with simulation mode before live trading

---

## 📋 Full Example Configuration

```env
# Core
RPC_URL=https://mainnet.helius-rpc.com/?api-key=xxx
KEYPAIR_B58=your_base58_key

# Mode
BOT_MODE=simulation
SIMULATION_BALANCE=10.0

# Mining
MINING_ENABLED=true
MINING_STRATEGY=weighted
DEPLOY_AMOUNT_SOL=0.1
MIN_SOL_BALANCE=0.5

# Betting (disabled for simulation)
BETTING_ENABLED=false

# Analytics
ANALYTICS_ENABLED=true
HISTORY_DEPTH=200

# Logging
RUST_LOG=info
```

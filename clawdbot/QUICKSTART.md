# ClawdBot Quick Start Guide

## 🚀 Getting Started in 5 Minutes

### Step 1: Prerequisites

Make sure you have:
```bash
# Check Rust
rustc --version

# Check Solana CLI
solana --version

# Check your wallet
solana balance
```

### Step 2: Clone and Build

```bash
cd clawdbot
cargo build --release
```

This will take a few minutes the first time.

### Step 3: Configure

```bash
# Copy example config
cp config.example.json config.json

# Edit with your settings
nano config.json  # or use your preferred editor
```

**Minimum changes needed:**
- Set `keypair_path` to your wallet location
- Adjust `deploy_amount_sol` based on your budget
- Set `min_sol_balance` to keep as reserve

### Step 4: Test with Monitor Bot

Start with just monitoring to see how things work:

```bash
RUST_LOG=info ./target/release/monitor-bot
```

You should see:
- Current balance
- Round information
- Real-time updates

Press Ctrl+C to stop.

### Step 5: Run Analytics Bot

See historical data and predictions:

```bash
RUST_LOG=info ./target/release/analytics-bot
```

This will:
- Analyze past rounds
- Show square statistics
- Predict winning squares
- Export data to `analytics.json`

### Step 6: Enable Mining (Optional)

**⚠️ WARNING: This will spend SOL!**

1. Edit `config.json`:
```json
{
  "mining": {
    "enabled": true,
    "deploy_amount_sol": 0.01  // Start small!
  }
}
```

2. Run the miner:
```bash
RUST_LOG=info ./target/release/miner-bot
```

### Step 7: Enable Betting (Optional)

**⚠️ WARNING: Higher risk!**

1. Edit `config.json`:
```json
{
  "betting": {
    "enabled": true,
    "bet_percentage": 0.01,  // Start with 1%
    "risk_tolerance": 0.3    // Conservative
  }
}
```

2. Run the betting bot:
```bash
RUST_LOG=info ./target/release/betting-bot
```

## 🎯 Using the Helper Script

The easiest way to run bots:

```bash
./run.sh
```

This interactive menu lets you:
1. Run individual bots
2. Run all bots in tmux
3. Build and manage the project

## 📊 Understanding the Output

### Monitor Bot
```
💰 Balance increased by 0.0523 SOL (now 1.2345 SOL)
🎲 New round started: 1234 → 1235
📊 Current round: 1235
```

### Analytics Bot
```
╔═══════════════════════════════════════════════════════╗
║              ORE ANALYTICS DASHBOARD                  ║
╠═══════════════════════════════════════════════════════╣
║ Rounds Analyzed:                                   100 ║
║ Most Winning Square: #7                                ║
```

### Miner Bot
```
🔨 Miner bot started
📊 Current round: 1235
🎯 Selected squares: [7, 12, 18]
⛏️  Would deploy 0.1000 SOL to squares [7, 12, 18]
```

### Betting Bot
```
🎲 Betting bot started
🎲 New round detected: 1235
🎯 Placing bets:
  Square #7: 0.0333 SOL
  Square #12: 0.0333 SOL
  Square #18: 0.0333 SOL
💰 Total bet: 0.1000 SOL across 3 squares
```

## 🛡️ Safety Tips

1. **Start Small**: Use tiny amounts (0.01 SOL) to test
2. **Monitor First**: Run monitor bot for a day before mining
3. **Analyze**: Check analytics to understand patterns
4. **Set Limits**: Use `min_sol_balance` to protect your funds
5. **Use Devnet**: Test on devnet first if possible

## 🐛 Troubleshooting

### "Failed to load keypair"
```bash
# Check your keypair path
ls ~/.config/solana/id.json

# Or specify full path in config.json
"keypair_path": "/home/user/.config/solana/id.json"
```

### "Insufficient balance"
```bash
# Check your balance
solana balance

# Fund your wallet
solana airdrop 1  # Devnet only
```

### "RPC error"
```bash
# Try a different RPC in config.json
"rpc_url": "https://api.mainnet-beta.solana.com"

# Or use a private RPC for better reliability
```

### Bot not doing anything
- Check that `enabled: true` in config
- Verify sufficient SOL balance
- Check logs with `RUST_LOG=debug`

## 📈 Optimization Tips

### For Miners
1. Use `weighted` strategy for better odds
2. Start with small amounts per round
3. Enable automation for continuous mining
4. Set auto-claim threshold based on gas costs

### For Bettors
1. Start with `spread` strategy (safest)
2. Use low risk_tolerance (0.3-0.4)
3. Bet on 3-5 squares for diversification
4. Monitor win rates and adjust

### For Analytics
1. Increase `history_depth` for better predictions
2. Export data regularly for backup
3. Look for patterns in square statistics
4. Use predictions to inform betting/mining

## 🎮 Running Multiple Bots

### Using tmux (Recommended)
```bash
./run.sh
# Choose option 5

# Attach to see all bots
tmux attach -t clawdbot

# Detach without stopping (Ctrl+B then D)
```

### Using separate terminals
```bash
# Terminal 1
RUST_LOG=info ./target/release/monitor-bot

# Terminal 2
RUST_LOG=info ./target/release/analytics-bot

# Terminal 3
RUST_LOG=info ./target/release/miner-bot
```

### Using systemd (Advanced)
See `systemd-setup.md` for running as services.

## 📚 Next Steps

1. Read the full [README.md](README.md)
2. Understand the strategies in detail
3. Join the ORE community
4. Contribute improvements
5. Share your results!

## 🆘 Getting Help

- Check bot logs for errors
- Review configuration settings
- Verify Solana network status
- Test with small amounts first
- Open an issue on GitHub

## 💡 Pro Tips

1. **Diversify**: Run multiple strategies
2. **Log Everything**: Keep logs for analysis
3. **Stay Updated**: Watch for ORE protocol changes
4. **Be Patient**: Mining takes time
5. **Risk Management**: Never invest more than you can lose

---

**Happy Mining! ⛏️💎**

*Remember: This is experimental software. Test thoroughly before using with real funds!*

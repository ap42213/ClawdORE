# Testing Guide for ClawdBot

## 🧪 Testing Strategy

### 1. Local Testing (Devnet)
**Start here!** Test on Solana devnet before risking real funds.

#### Setup
```bash
cd clawdbot

# Get devnet SOL (free!)
solana airdrop 2 --url devnet

# Use devnet config
cp config.devnet.json config.json

# Update with your wallet
nano config.json
```

#### Run Monitor Bot (Safe)
```bash
cargo run --release --bin monitor-bot
```

**What to check:**
- ✅ Connects to devnet
- ✅ Reads wallet balance
- ✅ Fetches round data
- ✅ No errors in logs

#### Run Analytics Bot
```bash
cargo run --release --bin analytics-bot
```

**What to check:**
- ✅ Fetches historical rounds
- ✅ Calculates statistics
- ✅ Generates predictions
- ✅ Exports data (if enabled)

### 2. Paper Trading
Test strategies without risking funds.

#### Setup
```bash
cp config.test.json config.json
```

#### Simulate Betting
```rust
// In your test code
let strategy = BettingStrategy::new("kelly".to_string(), 0.5);
let squares = strategy.select_squares(5, &history, &current_round)?;

// Calculate what you WOULD bet
let bets = strategy.calculate_optimal_bets(
    &squares,
    1000.0, // hypothetical bankroll
    &probabilities,
    0.01,
    1.0
);

println!("Would bet: {:?}", bets);
// DON'T actually send transaction
```

### 3. Small Live Test (Devnet)
After paper trading looks good.

#### Setup
```bash
# Use devnet config with SMALL amounts
nano config.json
# Set bet_percentage: 0.01 (1%)
# Set max_bet_sol: 0.1
```

#### Run Betting Bot
```bash
cargo run --release --bin betting-bot
```

**Monitor:**
- 📊 Strategy performance
- 💰 Bankroll changes
- 📈 Win rate
- ⚠️ Errors

**Run for 24 hours, check results**

### 4. Mainnet Testing (CAREFUL!)
Only after successful devnet testing.

#### Pre-flight Checklist
- [ ] Tested on devnet for 1+ week
- [ ] Paper trading shows profit
- [ ] Analytics look good
- [ ] No errors in logs
- [ ] Risk management working
- [ ] Emergency stop tested

#### Setup
```bash
# Use mainnet config with SMALL amounts
cp config.mainnet.json config.json

# IMPORTANT: Start conservative!
nano config.json
# Set bet_percentage: 0.01 (1%)
# Set max_bet_sol: 0.1
# Set risk_tolerance: 0.2 (conservative)
```

#### Start with Monitor Only
```bash
cargo run --release --bin monitor-bot
```

**Run for 24 hours. If stable, proceed.**

#### Add Analytics
```bash
cargo run --release --bin analytics-bot
```

**Run for 1 week. Verify data quality.**

#### Start Betting (SMALL!)
```bash
# DOUBLE CHECK config first!
cat config.json | grep bet_percentage

cargo run --release --bin betting-bot
```

**Monitor closely for first 48 hours!**

## 🔍 Testing Scenarios

### Scenario 1: Connection Failures
**Test retry logic**
```bash
# Kill internet connection while running
# Bot should retry with exponential backoff
# Check logs for retry messages
```

### Scenario 2: Low Balance
**Test safety limits**
```bash
# Set min_sol_balance: 0.5
# Drain wallet to 0.4 SOL
# Bot should pause/stop
```

### Scenario 3: Rate Limiting
**Test RPC limits**
```bash
# Use free RPC endpoint
# Run multiple bots simultaneously
# Should handle rate limits gracefully
```

### Scenario 4: Transaction Failures
**Test error handling**
```bash
# Submit with insufficient SOL
# Should catch error and log
# Should NOT crash
```

### Scenario 5: Strategy Comparison
**Test different strategies**
```bash
# Run Kelly vs Martingale vs Random
# Compare over 100 rounds
# Calculate Sharpe ratios
```

## 📊 Metrics to Track

### Performance Metrics
- **Win Rate**: Should be > 40% for good strategies
- **ROI**: Target > 5% over 100 rounds
- **Sharpe Ratio**: > 1.0 is good, > 2.0 is excellent
- **Max Drawdown**: Should be < 20% of bankroll

### Operational Metrics
- **Uptime**: Should be > 99%
- **RPC Success Rate**: Should be > 95%
- **Transaction Success Rate**: Should be > 90%
- **Average Bet Execution Time**: < 5 seconds

## 🧪 Unit Tests

### Test Strategy Selection
```bash
cargo test strategy::tests --release
```

### Test Analytics
```bash
cargo test analytics::tests --release
```

### Test Client
```bash
cargo test client::tests --release
```

## 🎯 Integration Tests

### Test Full Workflow
```bash
# Create integration test
cargo test --test integration_test --release
```

### Test API Endpoints
```bash
cd clawdbot-api
cargo test --release
```

### Test Frontend
```bash
cd frontend
npm test
```

## 🚨 Emergency Procedures

### Stop All Bots
```bash
# Ctrl+C in each terminal
# Or kill processes
pkill -f "miner-bot|betting-bot|analytics-bot"
```

### Check Status
```bash
# View logs
tail -f clawdbot.log

# Check balance
solana balance

# Check recent transactions
solana transaction-history
```

### Recover from Error
```bash
# Check last transaction
solana confirm <signature>

# If stuck, restart with fresh config
cp config.example.json config.json
```

## 📝 Test Checklist

Before going live, verify:

### Configuration
- [ ] RPC URL is correct (mainnet/devnet)
- [ ] Wallet path is correct
- [ ] Amounts are conservative
- [ ] Risk limits are set
- [ ] Notifications configured

### Safety
- [ ] Min balance check works
- [ ] Max bet limit works
- [ ] Auto-stop on errors
- [ ] Transaction simulation enabled
- [ ] Rate limiting works

### Monitoring
- [ ] Logs are being written
- [ ] Metrics are tracked
- [ ] Alerts are configured
- [ ] Dashboard is accessible
- [ ] Backup wallet exists

### Performance
- [ ] Strategies tested on devnet
- [ ] Analytics show positive EV
- [ ] Win rate is acceptable
- [ ] Drawdown is manageable
- [ ] ROI is positive

## 🎓 Learning Resources

### Understanding Metrics

**Sharpe Ratio:**
- < 0: Losing money
- 0-1: Okay
- 1-2: Good
- 2+: Excellent

**Kelly Criterion:**
- Optimal bet size for maximizing growth
- Uses win probability and odds
- We use 50% Kelly for safety

**Maximum Drawdown:**
- Largest peak-to-trough decline
- Important for assessing risk
- Should be < 20% for safety

## 🔄 Continuous Testing

### Daily
- Check logs for errors
- Verify balance hasn't dropped unexpectedly
- Review win rate

### Weekly
- Calculate ROI
- Compare strategies
- Adjust parameters if needed
- Review Sharpe ratio

### Monthly
- Full performance analysis
- Strategy comparison
- Risk assessment
- Parameter optimization

## ⚠️ Red Flags

Stop immediately if:
- ❌ Win rate drops below 30%
- ❌ Drawdown exceeds 25%
- ❌ Frequent RPC errors
- ❌ Transaction failures > 10%
- ❌ Unexpected balance drops
- ❌ Negative ROI for 3+ days

## 🎉 Success Criteria

You're ready for mainnet when:
- ✅ 2+ weeks stable on devnet
- ✅ Win rate > 40%
- ✅ Positive ROI
- ✅ Sharpe ratio > 1.0
- ✅ No crashes
- ✅ Logs are clean
- ✅ Analytics are accurate

---

**Remember:** Start small, test thoroughly, scale gradually! 🚀

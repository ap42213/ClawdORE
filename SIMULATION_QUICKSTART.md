# 🎮 ORE Simulation - Quick Reference

## ✅ What Was Built

### New Modules
1. **`ore_round.rs`** - ORE round tracking system
   - Split vs Full ORE detection
   - Motherlode monitoring
   - Wallet performance tracking
   - Pattern analysis

2. **`simulation.rs`** - Paper trading engine
   - Monitors mainnet (read-only)
   - Simulates participation
   - Tracks performance
   - Zero transactions

3. **`simulation_bot.rs`** - Runnable bot
   - Complete simulation bot binary
   - Real-time monitoring
   - Statistics display
   - Results export

### Key Features
- ✅ **60-second round monitoring** - Real ORE timing
- ✅ **Split detection** - Tracks when ORE splits among participants
- ✅ **Full ORE tracking** - Monitors winner-takes-all rounds
- ✅ **Motherlode detection** - Predicts rare jackpot events
- ✅ **Paper trading** - Test strategies with zero risk
- ✅ **Performance analytics** - ROI, win rate, etc.

## 🚀 How to Use

### Start Simulation
```bash
cd /workspaces/ClawdORE/clawdbot
cargo run --release --bin simulation-bot
```

### Customize Config
```bash
nano config.simulation.json
# Adjust:
# - bet_percentage
# - risk_tolerance
# - squares_to_bet
# - strategy type
```

### View Results
```bash
# Real-time in terminal
# OR
cat simulation_results.json | jq .
```

## 📊 ORE Mechanics Explained

### Every 60 Seconds
```
Round N starts
↓
Players deploy SOL
↓
60 seconds pass
↓
Outcome determined
↓
Either:
  A) Split: Everyone gets ORE share
  B) Full ORE: One winner gets 1 ORE
  C) Motherlode: One winner gets BIG ORE (rare!)
```

### Tracking
Bot monitors:
- Which outcome type (A, B, or C)
- How many participants
- Distribution patterns
- Your simulated performance

### Motherlode
- Rare event (1-5% of rounds)
- Larger than normal ORE reward
- Bot predicts likelihood
- Alert when probable

## 🎯 Strategy Development Workflow

### Week 1: Data Collection
```bash
# Just run and observe
cargo run --release --bin simulation-bot
```
**Collect:** Round patterns, split %, motherlode frequency

### Week 2-3: Strategy Testing
```json
// Test different configs
{
  "betting": {
    "strategy": "kelly",  // Change this
    "bet_percentage": 0.05, // Adjust this
    "squares_to_bet": 3  // Modify this
  }
}
```

### Week 4+: Optimization
Compare results → Pick winner → Optimize parameters

### Ready for Mainnet
After proven profitable in simulation for 4+ weeks

## 📈 Key Metrics

### Split Percentage
- **70-80%**: Normal
- **Higher**: Safe but lower rewards
- **Lower**: More volatile, higher potential

### Your Performance
- **Win Rate**: % rounds profitable
- **ROI**: Total return on investment
- **ORE Balance**: Accumulated earnings
- **SOL Balance**: Remaining capital

### Motherlode
- **Frequency**: How often it appears
- **Likelihood**: Bot's prediction
- **Impact**: Huge when you win!

## 🎓 What You Learn

### Patterns
- Best times to participate
- Optimal bet sizes
- When motherlode likely
- Competition levels

### Strategy
- Which approach works best
- Risk vs reward tradeoffs
- Bankroll management
- When to increase/decrease bets

### Confidence
- 4+ weeks data → confident strategy
- Proven profitable → ready for mainnet
- Understanding → better decisions

## ⚡ Quick Commands

```bash
# Start simulation
cargo run --release --bin simulation-bot

# Different config
cargo run --release --bin simulation-bot config.custom.json

# Stop (Ctrl+C shows final stats)

# View results
cat simulation_results.json | jq .

# Compare strategies
diff strategy_A_results.json strategy_B_results.json
```

## 🚦 When to Go Live

### ✅ Ready if:
- 4+ weeks simulation
- 200+ rounds tracked
- Positive simulated ROI
- Win rate > 40%
- Understand patterns
- Confident in strategy

### ⚠️ Start Small:
```json
{
  "mode": "live",
  "betting": {
    "max_bet_sol": 0.01,  // 1 cent!
    "bet_percentage": 0.01  // 1%
  }
}
```

## 💡 Pro Tips

1. **Run 24/7** - More data = better insights
2. **Test multiple strategies** - A/B compare
3. **Track motherlode accuracy** - Improve predictions
4. **Note patterns** - Time of day, day of week
5. **Be patient** - 4+ weeks minimum
6. **Export regularly** - Backup your data
7. **Don't rush mainnet** - Simulation is free!

## 🎉 The Best Part

### Zero Risk Learning
- ❌ No SOL needed
- ❌ No losses possible
- ✅ Real data
- ✅ Real patterns
- ✅ Real learning

### Ready When You Are
- Simulation proves strategy
- Then tiny mainnet test
- Then scale gradually
- All based on data!

---

## 📁 Files Created

- `ore_round.rs` - Round tracking system
- `simulation.rs` - Simulation engine
- `simulation_bot.rs` - Runnable bot
- `config.simulation.json` - Configuration
- `SIMULATION_GUIDE.md` - Full documentation

## 🆘 Need Help?

1. Check [SIMULATION_GUIDE.md](SIMULATION_GUIDE.md) - Full details
2. Review `config.simulation.json` - Configuration options
3. Check terminal output - Real-time stats
4. View `simulation_results.json` - Detailed data

---

**TL;DR:** Run simulation bot → Monitor real mainnet ORE → Track splits/full/motherlode → Test strategies → Go live when ready! 🚀

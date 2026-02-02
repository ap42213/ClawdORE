# ClawdBot - ORE Mining & Betting Bot System

A comprehensive bot system for mining ORE on ore.supply, placing strategic bets, and analyzing past rounds.

## Features

### 🔨 Miner Bot
- Automated ORE mining with configurable strategies
- Smart square selection based on historical data
- Automatic reward claiming
- Balance management and safety checks
- Automation support for continuous mining

### 🎲 Betting Bot
- Strategic betting across multiple squares
- Multiple betting strategies:
  - **Spread**: Distribute bets evenly
  - **Focused**: Concentrate on high-probability squares
  - **Hot Squares**: Follow recent winners
  - **Contrarian**: Bet against the crowd
  - **Weighted**: Data-driven probability betting
- Configurable risk tolerance
- Dynamic bet sizing

### 📊 Analytics Bot
- Real-time round analysis
- Historical performance tracking
- Square win rate calculations
- Prediction of likely winning squares
- Data export to JSON
- Beautiful dashboard display

### 📡 Monitor Bot
- Real-time balance monitoring
- Round change notifications
- Competition tracking
- Customizable alerts
- Low balance warnings

## Installation

### Prerequisites
- Rust 1.70 or higher
- Solana CLI tools
- A funded Solana wallet

### Build

```bash
cd clawdbot
cargo build --release
```

## Configuration

Create a `config.json` file:

```json
{
  "rpc_url": "https://api.mainnet-beta.solana.com",
  "ws_url": "wss://api.mainnet-beta.solana.com",
  "keypair_path": "~/.config/solana/id.json",
  "mining": {
    "enabled": true,
    "deploy_amount_sol": 0.1,
    "use_automation": true,
    "max_automation_balance": 1.0,
    "min_sol_balance": 0.5,
    "auto_claim_threshold_ore": 10.0,
    "strategy": "weighted"
  },
  "betting": {
    "enabled": true,
    "bet_percentage": 0.05,
    "max_bet_sol": 0.5,
    "min_bet_sol": 0.01,
    "risk_tolerance": 0.5,
    "squares_to_bet": 3,
    "strategy": "weighted"
  },
  "analytics": {
    "enabled": true,
    "history_depth": 100,
    "update_interval": 60,
    "use_database": false,
    "database_path": "./bot_data.db",
    "export_path": "./analytics.json"
  },
  "monitor": {
    "enabled": true,
    "check_interval": 30,
    "track_balance": true,
    "track_rounds": true,
    "track_competition": true,
    "alerts": {
      "min_balance_sol": 0.1,
      "round_ending_warning": 300,
      "large_win_threshold": 100.0
    }
  }
}
```

## Usage

### Run Individual Bots

#### Miner Bot
```bash
RUST_LOG=info ./target/release/miner-bot
```

#### Betting Bot
```bash
RUST_LOG=info ./target/release/betting-bot
```

#### Analytics Bot
```bash
RUST_LOG=info ./target/release/analytics-bot
```

#### Monitor Bot
```bash
RUST_LOG=info ./target/release/monitor-bot
```

### Run All Bots Together

You can run multiple bots in separate terminals or use a process manager like `tmux` or `screen`.

Example with tmux:
```bash
# Create a new tmux session
tmux new-session -d -s clawdbot

# Split into panes
tmux split-window -h -t clawdbot
tmux split-window -v -t clawdbot
tmux split-window -v -t clawdbot:0.0

# Run bots in each pane
tmux send-keys -t clawdbot:0.0 'RUST_LOG=info ./target/release/miner-bot' C-m
tmux send-keys -t clawdbot:0.1 'RUST_LOG=info ./target/release/betting-bot' C-m
tmux send-keys -t clawdbot:0.2 'RUST_LOG=info ./target/release/analytics-bot' C-m
tmux send-keys -t clawdbot:0.3 'RUST_LOG=info ./target/release/monitor-bot' C-m

# Attach to the session
tmux attach -t clawdbot
```

## Strategies

### Mining Strategies

- **random**: Randomly select squares (baseline)
- **weighted**: Select squares with lower deployment (better odds)
- **balanced**: Mix of low and medium deployment squares

### Betting Strategies

- **spread**: Evenly distribute bets across selected squares
- **focused**: Concentrate bets on highest probability squares
- **hot_squares**: Follow recent winning patterns
- **contrarian**: Bet against crowd behavior
- **weighted**: Data-driven probability-based betting

## Safety Features

- Minimum balance checks
- Maximum bet limits
- Automatic pause on errors
- Balance monitoring and alerts
- Transaction retry logic
- Configurable risk parameters

## Architecture

```
clawdbot/
├── src/
│   ├── lib.rs              # Library exports
│   ├── bot.rs              # Bot trait and runner
│   ├── client.rs           # Solana client wrapper
│   ├── config.rs           # Configuration structures
│   ├── error.rs            # Error types
│   ├── strategy.rs         # Mining and betting strategies
│   ├── analytics.rs        # Analytics engine
│   ├── monitor.rs          # Monitoring bot
│   └── bin/
│       ├── miner_bot.rs    # Miner bot binary
│       ├── betting_bot.rs  # Betting bot binary
│       ├── analytics_bot.rs # Analytics bot binary
│       └── monitor_bot.rs  # Monitor bot binary
└── Cargo.toml
```

## Performance Tips

1. **RPC Selection**: Use a reliable RPC endpoint or run your own validator
2. **Network**: Lower latency = faster transactions
3. **Balance Management**: Keep sufficient SOL for gas fees
4. **Strategy Tuning**: Adjust risk tolerance based on performance
5. **Monitoring**: Always run the monitor bot to track performance

## Troubleshooting

### Bot won't start
- Check keypair path is correct
- Verify RPC endpoint is accessible
- Ensure sufficient SOL balance

### Transactions failing
- Check network congestion
- Increase priority fees
- Verify account initialization

### Analytics not updating
- Check RPC rate limits
- Verify round data availability
- Check export path permissions

## Roadmap

- [ ] Machine learning-based predictions
- [ ] Advanced Kelly Criterion betting
- [ ] Multi-wallet support
- [ ] Web dashboard
- [ ] Telegram/Discord notifications
- [ ] Backtesting framework
- [ ] Database persistence
- [ ] Portfolio optimization
- [ ] Risk management tools
- [ ] Paper trading mode

## Security

- Never share your private keys
- Use environment variables for sensitive data
- Test on devnet first
- Start with small amounts
- Monitor bot activity regularly
- Keep dependencies updated

## Contributing

Contributions are welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

MIT License - See LICENSE file for details

## Disclaimer

This bot is for educational purposes. Cryptocurrency trading involves risk. Only invest what you can afford to lose. The authors are not responsible for any financial losses.

## Support

For issues and questions:
- Open an issue on GitHub
- Check existing documentation
- Review the code comments

## Acknowledgments

- Built on the ORE protocol by Regolith Labs
- Uses Solana blockchain technology
- Inspired by the mining and betting community

---

**Happy Mining! ⛏️💎**

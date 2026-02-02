# ClawdBot Web Terminal

A browser-based terminal interface for controlling ClawdBot bots.

## Features

- 🌐 Access from any browser
- 🎮 Control all bots with a GUI
- 📺 Real-time terminal output
- 🔄 Start/stop bots individually
- 📊 Live status monitoring

## Quick Start

### 1. Build the ClawdBots first

```bash
cd ../clawdbot
cargo build --release
```

### 2. Run the Web Server

```bash
cd ../clawdbot-web
cargo run --release
```

### 3. Open Your Browser

Navigate to: `http://localhost:3000`

## Usage

1. **Click "Start"** on any bot to launch it
2. **Watch the terminal** for real-time output
3. **Click "Stop"** to terminate a bot
4. **Multiple bots** can run simultaneously

## Available Bots

- **Monitor Bot** 📡 - Tracks balance and rounds (safe)
- **Analytics Bot** 📊 - Analyzes historical data (safe)
- **Miner Bot** ⛏️ - Mines ORE automatically (spends SOL)
- **Betting Bot** 🎲 - Places strategic bets (spends SOL)

## Screenshots

```
┌─────────────────────────────────────────────┐
│  🤖 ClawdBot Web Terminal                   │
├─────────────────┬───────────────────────────┤
│ Bot Controls    │ Terminal Output           │
│                 │                           │
│ ○ Monitor Bot   │ $ Starting monitor bot... │
│   [Start][Stop] │ ✓ Connected               │
│                 │ 💰 Balance: 2.5 SOL       │
│ ○ Analytics Bot │ 🎲 Round: #1234           │
│   [Start][Stop] │                           │
│                 │                           │
└─────────────────┴───────────────────────────┘
```

## Configuration

The web terminal uses the same `config.json` from the clawdbot directory:

```bash
# Make sure this exists
ls ../clawdbot/config.json
```

## Port Configuration

Default port: `3000`

To change, edit `src/main.rs`:
```rust
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
```

## Troubleshooting

**"Bot binary not found"**
```bash
cd ../clawdbot && cargo build --release
```

**Can't connect to server**
- Check the terminal for errors
- Ensure port 3000 is available
- Try: `lsof -i :3000`

**Bots won't start**
- Verify `config.json` exists in clawdbot/
- Check wallet is configured
- Review logs in the terminal

## Development

Run in development mode:
```bash
cargo run
```

Build for production:
```bash
cargo build --release
```

## Security Note

⚠️ This web interface runs locally. Do not expose it to the internet without proper authentication!

## Future Features

- [ ] Real-time stats dashboard
- [ ] Bot output streaming
- [ ] Configuration editor
- [ ] Performance charts
- [ ] Mobile responsive design
- [ ] Dark/light theme toggle

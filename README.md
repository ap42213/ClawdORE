# 🦞 ClawdORE - Autonomous ORE Mining Intelligence Swarm

> **🏆 Colosseum Agent Hackathon 2026 Submission**
> 
> A 7-bot autonomous intelligence system for ORE mining on Solana, built entirely by AI agents.

[![Built by AI](https://img.shields.io/badge/Built%20by-AI%20Agents-blueviolet)](https://colosseum.com/agent-hackathon)
[![Solana](https://img.shields.io/badge/Solana-Mainnet-14F195)](https://solana.com)
[![ORE](https://img.shields.io/badge/ORE-v3-orange)](https://ore.supply)

## 🤖 The Swarm

ClawdORE deploys **7 specialized bots** that coordinate via PostgreSQL to make intelligent mining decisions:

| Bot | Name | Role |
|-----|------|------|
| 🎯 | **CLAWDOREDINATOR** | Central coordinator - orchestrates all bot signals |
| ⛏️ | **MINEORE** | Mining executor - handles ORE mining operations |
| 👁️ | **MONITORE** | Network monitor - tracks chain state & round timing |
| 📊 | **ANALYTICORE** | Analytics engine - pattern detection & statistics |
| 🔍 | **PARSEORE** | Transaction parser - decodes on-chain activity |
| 🧠 | **LEARNORE** | Machine learning - adaptive strategy optimization |
| 🎰 | **BETORE** | Betting intelligence - prediction & wagering |

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    PostgreSQL Database                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   signals   │  │    state    │  │   bot_heartbeats    │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
         ▲                 ▲                    ▲
         │                 │                    │
    ┌────┴────┐       ┌────┴────┐          ┌────┴────┐
    │ PARSEORE│       │MONITORE │          │LEARNORE │
    │ ANALYTI-│       │CLAWDORE-│          │ BETORE  │
    │  CORE   │       │ DINATOR │          │ MINEORE │
    └─────────┘       └─────────┘          └─────────┘
```

## 🚀 Deployment

### Railway (7 Bot Services + PostgreSQL)

Each bot runs as a separate Railway service with shared PostgreSQL:

```bash
# Environment variables per service:
DATABASE_URL=postgresql://...
BOT_TYPE=coordinator-bot  # or miner-bot, monitor-bot, etc.
RPC_URL=https://api.mainnet-beta.solana.com
```

### Frontend Dashboard (Vercel)

Real-time visualization of bot signals and network state:

```bash
cd frontend
npm install
vercel deploy
```

## 🎮 Features

### Intelligent Coordination
- **Signal aggregation** - All bots contribute signals to shared database
- **Consensus voting** - Coordinator synthesizes recommendations
- **Real-time state** - Live network monitoring with 60-second rounds

### ORE Protocol Integration
- **Program ID**: `oreV3EG1i9BEgiAJ8b177Z2S2rMarzak4NMv1kULvWv`
- **Round tracking** - Monitor 60-second mining rounds
- **Outcome detection** - Split ORE vs Full ORE vs Motherlode
- **On-chain parsing** - Decode mining transactions

### Machine Learning
- **Pattern detection** - Historical outcome analysis
- **Adaptive strategies** - Self-improving betting algorithms
- **Risk management** - Dynamic position sizing

## 📊 Signal Types

Bots emit signals to the shared database:

```rust
pub enum SignalType {
    MineRecommendation,    // Which square to mine
    NetworkState,          // Round timing & status
    PatternDetected,       // Historical pattern match
    RiskAlert,             // Risk threshold breach
    ConsensusReached,      // Bots agree on action
}
```

## 🛠️ Tech Stack

- **Language**: Rust 1.85
- **Database**: PostgreSQL (Railway)
- **Blockchain**: Solana Mainnet
- **Frontend**: Next.js 14 + TypeScript + Tailwind
- **Deployment**: Railway (bots) + Vercel (dashboard)

## 📁 Project Structure

```
ClawdORE/
├── clawdbot/                 # Core bot system
│   ├── src/
│   │   ├── lib.rs            # Shared library
│   │   ├── bot.rs            # Bot trait & runner
│   │   ├── client.rs         # Solana/ORE client
│   │   ├── strategy.rs       # Mining strategies
│   │   ├── analytics.rs      # Pattern analysis
│   │   └── bin/              # 7 bot binaries
│   │       ├── coordinator_bot.rs
│   │       ├── miner_bot.rs
│   │       ├── monitor_bot.rs
│   │       ├── analytics_bot.rs
│   │       ├── parser_bot.rs
│   │       ├── learning_bot.rs
│   │       └── betting_bot.rs
│   └── Cargo.toml
├── frontend/                 # Dashboard
│   ├── app/
│   │   ├── page.tsx
│   │   ├── api/              # PostgreSQL API routes
│   │   └── components/
│   └── package.json
├── Dockerfile                # Multi-bot container
└── railway.json              # Railway config
```

## 🔒 Security

- Database credentials via environment variables
- Read-only RPC access (no private keys in bots)
- Rate limiting on API endpoints
- Automatic restart on failure

## ⚠️ Disclaimer

This software is for educational and hackathon purposes. Cryptocurrency mining involves risk. The authors are not responsible for any financial losses.

## 🏆 Hackathon

**Colosseum Agent Hackathon 2026**
- **Dates**: February 2-12, 2026
- **Agent**: ClawdORE (ID: 82)
- **Tags**: `ai`, `depin`

Built entirely by AI agents per hackathon rules.

## 📜 License

MIT License

---

**Built with 🦞 by ClawdORE AI Swarm**

*Mining smarter, together.* ⛏️🤖

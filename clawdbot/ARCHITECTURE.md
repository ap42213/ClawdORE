# ClawdBot Architecture

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         ClawdBot System                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │ Monitor Bot │  │ Analytics   │  │  Miner Bot  │            │
│  │             │  │     Bot     │  │             │            │
│  │ • Balance   │  │ • History   │  │ • Deploy    │            │
│  │ • Rounds    │  │ • Analysis  │  │ • Claim     │            │
│  │ • Alerts    │  │ • Predict   │  │ • Automate  │            │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘            │
│         │                 │                 │                    │
│         │                 │                 │                    │
│         └─────────────────┼─────────────────┘                    │
│                           │                                      │
│                    ┌──────▼──────┐                               │
│                    │  OreClient  │                               │
│                    │             │                               │
│                    │ • RPC Calls │                               │
│                    │ • Accounts  │                               │
│                    │ • Tx Send   │                               │
│                    └──────┬──────┘                               │
│                           │                                      │
└───────────────────────────┼──────────────────────────────────────┘
                            │
                            ▼
            ┌───────────────────────────┐
            │   Solana Blockchain        │
            ├───────────────────────────┤
            │                            │
            │  ┌──────────────────────┐ │
            │  │   ORE Program        │ │
            │  │                      │ │
            │  │  • Board             │ │
            │  │  • Rounds            │ │
            │  │  • Miners            │ │
            │  │  • Treasury          │ │
            │  └──────────────────────┘ │
            │                            │
            └────────────────────────────┘
```

## Data Flow

### Mining Operation

```
User Config
    │
    ▼
┌─────────────────┐
│  Miner Bot      │
└────────┬────────┘
         │
         ├─→ Get Board State
         │   (current round)
         │
         ├─→ Get Round Data
         │   (deployment info)
         │
         ├─→ Analyze History
         │   (past rounds)
         │
         ├─→ Select Strategy
         │   (weighted, random, etc)
         │
         ├─→ Choose Squares
         │   (1-N squares)
         │
         ├─→ Build Transaction
         │   (deploy instruction)
         │
         ├─→ Sign & Send
         │   (to blockchain)
         │
         └─→ Monitor Result
             (update state)
```

### Betting Operation

```
User Config
    │
    ▼
┌─────────────────┐
│  Betting Bot    │
└────────┬────────┘
         │
         ├─→ Wait for New Round
         │
         ├─→ Get Historical Data
         │   (N past rounds)
         │
         ├─→ Analyze Patterns
         │   (win rates, trends)
         │
         ├─→ Calculate Risk
         │   (based on tolerance)
         │
         ├─→ Select Squares
         │   (M squares)
         │
         ├─→ Size Positions
         │   (% of bankroll)
         │
         ├─→ Place Bets
         │   (deploy to squares)
         │
         └─→ Track Results
             (win/loss tracking)
```

### Analytics Operation

```
User Config
    │
    ▼
┌─────────────────┐
│ Analytics Bot   │
└────────┬────────┘
         │
         ├─→ Fetch Rounds
         │   (last N rounds)
         │
         ├─→ Store History
         │   (in memory/db)
         │
         ├─→ Calculate Stats
         │   • Win rates
         │   • Deployment avg
         │   • ROI
         │
         ├─→ Generate Predictions
         │   (ML/statistical)
         │
         ├─→ Create Reports
         │   (dashboard display)
         │
         └─→ Export Data
             (JSON/CSV/DB)
```

## Component Architecture

### Bot Structure

```
┌──────────────────────────────────────┐
│           Bot Trait                  │
├──────────────────────────────────────┤
│                                      │
│  + name() -> &str                   │
│  + status() -> BotStatus            │
│  + start() -> Result<()>            │
│  + stop() -> Result<()>             │
│  + pause() -> Result<()>            │
│  + resume() -> Result<()>           │
│                                      │
└──────────────────────────────────────┘
              ▲
              │ implements
              │
    ┌─────────┴──────────┬──────────────────┬──────────────────┐
    │                    │                  │                  │
┌───┴────┐         ┌─────┴──────┐    ┌─────┴──────┐    ┌─────┴──────┐
│ Miner  │         │  Betting   │    │ Analytics  │    │  Monitor   │
│  Bot   │         │    Bot     │    │    Bot     │    │    Bot     │
└────────┘         └────────────┘    └────────────┘    └────────────┘
```

### Strategy System

```
┌──────────────────────────────────────┐
│         Strategy Interface           │
├──────────────────────────────────────┤
│  select_squares(...)                 │
└──────────────────────────────────────┘
              ▲
              │
    ┌─────────┴──────────┬──────────────────┐
    │                    │                  │
┌───┴────────────┐  ┌────┴─────────┐  ┌────┴──────────┐
│    Mining      │  │   Betting    │  │   Custom      │
│   Strategy     │  │   Strategy   │  │   Strategy    │
│                │  │              │  │               │
│ • Random       │  │ • Spread     │  │ • Your own    │
│ • Weighted     │  │ • Focused    │  │   logic       │
│ • Balanced     │  │ • Contrarian │  │               │
└────────────────┘  └──────────────┘  └───────────────┘
```

### Analytics Engine

```
┌──────────────────────────────────────────────────┐
│           Analytics Engine                       │
├──────────────────────────────────────────────────┤
│                                                  │
│  ┌────────────────┐                             │
│  │ Round History  │◄─────── add_round()         │
│  │   Storage      │                             │
│  └───────┬────────┘                             │
│          │                                       │
│          ├─→ analyze_rounds()                   │
│          │   → RoundAnalytics[]                 │
│          │                                       │
│          ├─→ analyze_squares()                  │
│          │   → SquareStatistics[]               │
│          │                                       │
│          ├─→ predict_winning_squares()          │
│          │   → Vec<usize>                       │
│          │                                       │
│          └─→ get_overall_analytics()            │
│              → OverallAnalytics                  │
│                                                  │
└──────────────────────────────────────────────────┘
```

## State Management

```
┌─────────────────────────────────────────┐
│         Bot State                       │
├─────────────────────────────────────────┤
│                                         │
│  Arc<RwLock<BotStatus>>                │
│         │                               │
│         ├─→ Idle                        │
│         ├─→ Running                     │
│         ├─→ Paused                      │
│         ├─→ Stopped                     │
│         └─→ Error                       │
│                                         │
│  Arc<RwLock<Data>>                     │
│         │                               │
│         ├─→ Last Balance                │
│         ├─→ Last Round                  │
│         ├─→ Statistics                  │
│         └─→ Performance                 │
│                                         │
└─────────────────────────────────────────┘
```

## Concurrency Model

```
Main Thread
    │
    ├─→ Load Config
    │
    ├─→ Create Clients
    │
    ├─→ Initialize Bots
    │
    └─→ Start BotRunner
            │
            ├─→ [Tokio Task] Monitor Bot
            │       └─→ Loop every 30s
            │
            ├─→ [Tokio Task] Analytics Bot
            │       └─→ Loop every 60s
            │
            ├─→ [Tokio Task] Miner Bot
            │       └─→ Loop continuously
            │
            └─→ [Tokio Task] Betting Bot
                    └─→ Wait for new rounds
```

## Configuration Hierarchy

```
config.json
    │
    ├─→ Global Settings
    │   ├─ rpc_url
    │   ├─ ws_url
    │   └─ keypair_path
    │
    ├─→ Mining Config
    │   ├─ enabled
    │   ├─ deploy_amount
    │   ├─ strategy
    │   └─ thresholds
    │
    ├─→ Betting Config
    │   ├─ enabled
    │   ├─ risk_tolerance
    │   ├─ strategy
    │   └─ limits
    │
    ├─→ Analytics Config
    │   ├─ history_depth
    │   ├─ update_interval
    │   └─ export_path
    │
    └─→ Monitor Config
        ├─ check_interval
        ├─ tracking options
        └─ alerts
```

## Error Handling Flow

```
Operation
    │
    ├─→ Try Execute
    │       │
    │       ├─ Success → Return Ok(T)
    │       │
    │       └─ Failure
    │           │
    │           ├─→ Map Error
    │           │   (to BotError variant)
    │           │
    │           ├─→ Log Error
    │           │   (with context)
    │           │
    │           ├─→ Update Status
    │           │   (if needed)
    │           │
    │           └─→ Return Err(BotError)
    │
    └─→ Caller Handles
        │
        ├─→ Retry?
        ├─→ Fallback?
        ├─→ Alert?
        └─→ Shutdown?
```

## Security Model

```
┌─────────────────────────────────────────┐
│         Security Layers                 │
├─────────────────────────────────────────┤
│                                         │
│  1. Configuration                       │
│     └─ .gitignore excludes keypairs    │
│                                         │
│  2. Balance Protection                  │
│     ├─ min_balance checks              │
│     └─ max_bet limits                  │
│                                         │
│  3. Transaction Validation              │
│     ├─ Verify amounts                  │
│     └─ Check accounts                  │
│                                         │
│  4. Error Recovery                      │
│     ├─ Retry logic                     │
│     └─ Graceful degradation            │
│                                         │
│  5. Monitoring                          │
│     ├─ Balance alerts                  │
│     └─ Unusual activity detection      │
│                                         │
└─────────────────────────────────────────┘
```

## Deployment Options

### Single Bot (Simple)

```
User → Bot Binary → Solana
```

### Multiple Bots (Terminal)

```
Terminal 1 → Monitor Bot  ──┐
Terminal 2 → Analytics Bot ─┤
Terminal 3 → Miner Bot    ──┼→ Solana
Terminal 4 → Betting Bot  ──┘
```

### Tmux (Recommended)

```
Tmux Session "clawdbot"
├─ Pane 0: Monitor Bot
├─ Pane 1: Analytics Bot
├─ Pane 2: Miner Bot
└─ Pane 3: Betting Bot
         │
         └─→ All connect to Solana
```

### Systemd (Production)

```
systemd
├─ clawdbot-monitor.service
├─ clawdbot-analytics.service
├─ clawdbot-miner.service
└─ clawdbot-betting.service
         │
         └─→ Managed restarts
```

---

This architecture provides:
- **Modularity**: Each bot is independent
- **Scalability**: Easy to add new bots
- **Reliability**: Error isolation
- **Flexibility**: Multiple deployment options
- **Maintainability**: Clear separation of concerns

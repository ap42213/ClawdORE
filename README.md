# ClawdBot - Railway + Vercel Deployment

🤖 **Production-ready ORE mining & betting bot system**

## 🚀 Quick Deploy

### Backend (Railway)
```bash
cd clawdbot-api
railway login
railway init
railway up
```

### Frontend (Vercel)
```bash
cd frontend
npm install
vercel login
vercel
```

## 📦 Project Structure

```
ClawdORE/
├── clawdbot/           # Core bot system (Rust)
│   ├── src/
│   │   ├── bot.rs      # Bot trait & runner
│   │   ├── client.rs   # Solana/ORE client
│   │   ├── strategy.rs # Mining & betting strategies
│   │   └── bin/        # 4 specialized bots
│   └── Cargo.toml
│
├── clawdbot-api/       # REST API for Railway (Rust)
│   ├── src/main.rs     # Bot management API
│   └── Cargo.toml
│
├── frontend/           # Dashboard for Vercel (Next.js)
│   ├── app/
│   │   ├── page.tsx    # Main dashboard
│   │   └── components/ # UI components
│   └── package.json
│
└── docs/               # Comprehensive documentation
    ├── DEPLOYMENT.md   # Full deployment guide
    ├── RAILWAY.md      # Railway-specific docs
    └── API.md          # API reference
```

## 🎯 Features

### 4 Specialized Bots
1. **Monitor Bot** - Tracks wallet balance & round status
2. **Analytics Bot** - Analyzes patterns & predicts outcomes
3. **Miner Bot** - Automated ORE mining
4. **Betting Bot** - Strategic betting on squares
5. **🎮 Simulation Bot** - Paper trade with real mainnet data (NEW!)

### ORE Protocol Monitoring
- ✅ **60-second rounds** - Real-time tracking
- ✅ **Split vs Full ORE** - Outcome analysis
- ✅ **Motherlode detection** - Rare event monitoring
- ✅ **Paper trading** - Zero-risk strategy testing
- ✅ **Pattern analysis** - Historical data insights

### Modern Stack
- **Backend**: Rust + Axum (fast & reliable)
- **Frontend**: Next.js 14 + TypeScript + Tailwind
- **Deploy**: Railway (backend) + Vercel (frontend)
- **Blockchain**: Solana + ORE protocol

## 📚 Documentation

- **[SIMULATION_GUIDE.md](SIMULATION_GUIDE.md)** - 🎮 Paper trading guide (START HERE!)
- **[SIMULATION_QUICKSTART.md](SIMULATION_QUICKSTART.md)** - Quick reference
- **[DEPLOYMENT.md](DEPLOYMENT.md)** - Complete deployment guide
- **[RAILWAY.md](RAILWAY.md)** - Railway-specific instructions
- **[TESTING.md](TESTING.md)** - Testing procedures
- **[IMPROVEMENTS.md](IMPROVEMENTS.md)** - Technical improvements
- **[clawdbot/README.md](clawdbot/README.md)** - Bot system docs
- **[frontend/README.md](frontend/README.md)** - Frontend docs
- **[clawdbot-api/README.md](clawdbot-api/README.md)** - API docs

## 🔧 Local Development

### 1. Start with Simulation (Recommended!)
```bash
cd clawdbot
cargo run --release --bin simulation-bot
```
**Paper trade with real mainnet data - zero risk!**

### 2. Run API
```bash
cd clawdbot-api
cargo run
```

### 3. Run Frontend
```bash
cd frontend
npm install
npm run dev
```

### 4. Build Bots
```bash
cd clawdbot
cargo build --release
```

## 🌐 Production URLs

After deployment:
- Frontend: `https://your-project.vercel.app`
- API: `https://your-project.railway.app`

## 🔒 Environment Variables

### Railway (API + Bots)
```env
RUST_LOG=info
PORT=3000
SOLANA_KEYPAIR=<base64-encoded-keypair>
RPC_URL=https://api.mainnet-beta.solana.com
```

### Vercel (Frontend)
```env
NEXT_PUBLIC_API_URL=https://your-api.railway.app
```

## 💰 Cost Estimate

- **Railway**: Free tier ($5 credit/month)
- **Vercel**: Free tier (unlimited requests)
- **Start for $0!**

## 🛡️ Safety Features

- Paper trading mode
- Risk management
- Balance monitoring
- Automatic cooldowns
- Error handling

## � Dashboard Features

- Real-time bot status
- Live terminal logs
- Statistics dashboard
- One-click start/stop
- Res🎮 Start with Simulation** (ZERO RISK!)
   ```bash
   cd clawdbot
   cargo run --release --bin simulation-bot
   ```
   Paper trade with real mainnet ORE data!

2. **Read [SIMULATION_GUIDE.md](SIMULATION_GUIDE.md)**
3. **Test strategies for 4+ weeks**
4. **Then consider live deployment** (Railway + Vercel)
5. **Start micro-stakes** (0.01 SOL) when ready
2. **Read [DEPLOYMENT.md](DEPLOYMENT.md)**
3. **Deploy API to Railway**
4. **Deploy Frontend to Vercel**
5. **Start with Monitor Bot** (safest)

## ⚠️ Important Notes

- Start with **devnet** for testing
- Monitor Bot is safest (no spending)
- Miner/Betting bots cost SOL
- Always monitor logs
- Set spending limits

## 🤝 Support

- Issues: GitHub Issues
- Docs: See documentation files
- ORE Protocol: [ore.supply](https://ore.supply)

## 📝 License

MIT License

## 🎉 Quick Links

- [Railway Dashboard](https://railway.app)
- [Vercel Dashboard](https://vercel.com)
- [ORE Supply](https://ore.supply)
- [Solana Explorer](https://explorer.solana.com)

---

**Ready to deploy?** Start with [DEPLOYMENT.md](DEPLOYMENT.md)! 🚀
- Manage risk automatically
- Track ROI across rounds
- Optimize bet sizing

## 🛠️ Technology Stack

- **Language**: Rust
- **Blockchain**: Solana
- **Framework**: Anchor
- **Protocol**: ORE by Regolith Labs
- **Game**: OpenClaw-inspired mechanics

## 📊 Strategies Included

### Mining Strategies
- **Random** - Baseline comparison
- **Weighted** - Lower deployment = better odds
- **Balanced** - Mixed deployment approach

### Betting Strategies
- **Spread** - Diversified betting
- **Focused** - High-probability concentration
- **Hot Squares** - Follow recent winners
- **Contrarian** - Bet against the crowd
- **Weighted** - Data-driven probabilities

## 🔒 Security

- Private key management
- Balance protection
- Transaction verification
- Rate limiting
- Error recovery

## ⚠️ Disclaimer

This software is for educational purposes. Cryptocurrency mining and betting involve risk. Only use funds you can afford to lose. The authors are not responsible for any financial losses.

## 🤝 Contributing

Contributions welcome! Please:
1. Fork the repo
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a PR

## 📜 License

MIT License - See individual project licenses

## 🙏 Acknowledgments

- **OpenClaw** - Original Claw game developers
- **Regolith Labs** - ORE protocol creators
- **Solana Foundation** - Blockchain infrastructure
- **Anchor** - Solana development framework

## 🔗 Resources

- [ORE Website](https://ore.supply)
- [Solana Docs](https://docs.solana.com)
- [Anchor Docs](https://www.anchor-lang.com)
- [OpenClaw](https://github.com/openclaw/openclaw)

## 📞 Support

For ClawdBot issues:
- Open an issue on GitHub
- Check the documentation
- Review code comments

---

**Built with ❤️ for the ORE community**

*Start mining smarter, not harder! ⛏️💎*
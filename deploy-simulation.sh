#!/bin/bash

echo "🎮 ClawdBot Simulation - Railway Deployment"
echo "==========================================="
echo ""

# Check if Railway CLI is installed
if ! command -v railway &> /dev/null; then
    echo "📦 Installing Railway CLI..."
    npm i -g @railway/cli
fi

# Check if wallet exists
if [ ! -f "clawdbot/wallet.json" ]; then
    echo "🔑 Creating simulation wallet (no SOL needed)..."
    solana-keygen new -o clawdbot/wallet.json --no-bip39-passphrase
    echo "✅ Wallet created!"
else
    echo "✅ Wallet already exists"
fi

echo ""
echo "📡 Next steps:"
echo "1. Login to Railway:    railway login"
echo "2. Initialize project:  cd clawdbot && railway init"
echo "3. Set variables:       railway variables set RPC_URL=\"https://api.mainnet-beta.solana.com\""
echo "4. Deploy:              railway up"
echo ""
echo "💡 See RAILWAY_SIMULATION.md for full instructions"

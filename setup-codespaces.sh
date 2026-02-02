#!/bin/bash
# Quick setup script for GitHub Codespaces

set -e

echo "🚀 Setting up ClawdBot in Codespaces..."

# Install Rust if not present
if ! command -v cargo &> /dev/null; then
    echo "📦 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Install Solana CLI if not present
if ! command -v solana &> /dev/null; then
    echo "🔗 Installing Solana CLI..."
    sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
    export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
fi

echo "✅ Dependencies installed!"
echo ""
echo "🔨 Building ClawdBot..."
cd /workspaces/ClawdORE/clawdbot
cargo build --release

echo ""
echo "✅ Build complete!"
echo ""
echo "📝 Next steps:"
echo "1. Configure your bot: cp config.example.json config.json && nano config.json"
echo "2. Run a bot: RUST_LOG=info ./target/release/monitor-bot"
echo ""
echo "Or use the interactive runner: ./run.sh"

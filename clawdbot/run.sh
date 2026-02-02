#!/bin/bash
# ClawdBot Runner Script

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔═══════════════════════════════════════╗${NC}"
echo -e "${BLUE}║        ClawdBot Control Panel         ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════╝${NC}"
echo ""

# Check if config exists
if [ ! -f "config.json" ]; then
    echo -e "${YELLOW}⚠️  No config.json found. Creating from example...${NC}"
    cp config.example.json config.json
    echo -e "${GREEN}✓ Created config.json${NC}"
    echo -e "${YELLOW}Please edit config.json with your settings before running bots.${NC}"
    exit 1
fi

# Build if needed
if [ ! -f "target/release/miner-bot" ]; then
    echo -e "${BLUE}🔨 Building ClawdBot...${NC}"
    cargo build --release
    echo -e "${GREEN}✓ Build complete${NC}"
fi

# Function to run a bot
run_bot() {
    local bot_name=$1
    local bot_binary=$2
    
    echo -e "${GREEN}🤖 Starting $bot_name...${NC}"
    RUST_LOG=info ./target/release/$bot_binary
}

# Function to run all bots in tmux
run_all_tmux() {
    echo -e "${BLUE}🚀 Starting all bots in tmux session...${NC}"
    
    # Check if tmux is installed
    if ! command -v tmux &> /dev/null; then
        echo -e "${RED}❌ tmux is not installed. Please install it first.${NC}"
        exit 1
    fi
    
    # Kill existing session if it exists
    tmux kill-session -t clawdbot 2>/dev/null || true
    
    # Create new session
    tmux new-session -d -s clawdbot -n "ClawdBot"
    
    # Split into 4 panes
    tmux split-window -h -t clawdbot
    tmux split-window -v -t clawdbot:0.0
    tmux split-window -v -t clawdbot:0.1
    
    # Run bots in each pane
    tmux send-keys -t clawdbot:0.0 'RUST_LOG=info ./target/release/monitor-bot' C-m
    tmux send-keys -t clawdbot:0.1 'RUST_LOG=info ./target/release/analytics-bot' C-m
    tmux send-keys -t clawdbot:0.2 'RUST_LOG=info ./target/release/miner-bot' C-m
    tmux send-keys -t clawdbot:0.3 'RUST_LOG=info ./target/release/betting-bot' C-m
    
    echo -e "${GREEN}✓ All bots started in tmux session 'clawdbot'${NC}"
    echo -e "${YELLOW}Attach with: tmux attach -t clawdbot${NC}"
    echo -e "${YELLOW}Detach with: Ctrl+B then D${NC}"
    echo -e "${YELLOW}Kill session: tmux kill-session -t clawdbot${NC}"
}

# Main menu
echo "Select an option:"
echo "1) Run Monitor Bot"
echo "2) Run Analytics Bot"
echo "3) Run Miner Bot"
echo "4) Run Betting Bot"
echo "5) Run All Bots (tmux)"
echo "6) Build Only"
echo "7) Clean Build"
echo "8) Exit"
echo ""
read -p "Enter choice [1-8]: " choice

case $choice in
    1)
        run_bot "Monitor Bot" "monitor-bot"
        ;;
    2)
        run_bot "Analytics Bot" "analytics-bot"
        ;;
    3)
        run_bot "Miner Bot" "miner-bot"
        ;;
    4)
        run_bot "Betting Bot" "betting-bot"
        ;;
    5)
        run_all_tmux
        ;;
    6)
        echo -e "${BLUE}🔨 Building...${NC}"
        cargo build --release
        echo -e "${GREEN}✓ Build complete${NC}"
        ;;
    7)
        echo -e "${BLUE}🧹 Cleaning and rebuilding...${NC}"
        cargo clean
        cargo build --release
        echo -e "${GREEN}✓ Clean build complete${NC}"
        ;;
    8)
        echo -e "${BLUE}👋 Goodbye!${NC}"
        exit 0
        ;;
    *)
        echo -e "${RED}❌ Invalid option${NC}"
        exit 1
        ;;
esac

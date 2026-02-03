# ORE Regolith Dashboard

A professional real-time mining dashboard for ORE Regolith, built entirely in Rust using [Dioxus](https://dioxuslabs.com).

![Dashboard Preview](preview.png)

## Features

- **Real-time Grid Visualization**: Watch the 5x5 Regolith grid update live with deploy amounts
- **Accurate Round Timer**: Synced with actual Solana blockchain slots
- **Winner Reveal Animation**: Dramatic reveal when each round completes
- **Heat Map**: Visual intensity based on SOL deployed per square
- **Stats Dashboard**: Track rounds, SOL deployed, motherlodes, and more
- **Recent Rounds History**: View last 10 round winners
- **Professional Dark Theme**: Easy on the eyes for extended monitoring

## Tech Stack

- **Dioxus 0.6**: Modern Rust UI framework (React-like)
- **WebAssembly**: Runs entirely in browser
- **gloo**: Rust WASM utilities for timers and HTTP
- **serde**: JSON serialization

## Building

### Prerequisites

1. Install Rust: https://rustup.rs
2. Install Dioxus CLI:
   ```bash
   cargo install dioxus-cli
   ```

### Development

```bash
cd ore-dashboard
dx serve
```

Open http://localhost:8080 in your browser.

### Production Build

```bash
dx build --release
```

The built files will be in `target/dx/ore-dashboard/release/web/`.

## Deployment

### With clawdbot-web (recommended)

The dashboard is served by the `clawdbot-web` backend which also provides the API:

```bash
cd clawdbot-web
cargo run
```

The dashboard will be available at http://localhost:3000

### Standalone

Build the dashboard and serve the static files with any web server. You'll need to configure the API endpoint in `src/main.rs`:

```rust
const API_BASE_URL: &str = "https://your-api-server.com";
```

## API

The dashboard fetches data from `/api/state` which returns:

```json
{
  "board": {
    "round_id": 12345,
    "start_slot": 300000000,
    "end_slot": 300000150,
    "current_slot": 300000100,
    "deployed": [0, 100000000, 0, ...],
    "time_remaining_secs": 20,
    "round_duration_secs": 60,
    "slots_remaining": 50
  },
  "last_winner": {
    "round_id": 12344,
    "winning_square": 5,
    "total_pot": 1500000000,
    "is_motherlode": false
  },
  "stats": {
    "total_rounds_today": 1200,
    "total_sol_deployed": 150.5,
    "avg_round_time": 55.0,
    "motherlode_count": 12
  },
  "recent_rounds": [...]
}
```

## Customization

### Colors

Edit `assets/main.css` to customize the color scheme:

```css
:root {
    --accent-primary: #f7931a;    /* ORE Orange/Gold */
    --accent-success: #00d4aa;    /* Winner Green */
    --bg-primary: #0a0a0f;        /* Background */
}
```

### Fonts

The dashboard uses JetBrains Mono for code/numbers and Inter for text. Change in `index.html`:

```html
<link href="https://fonts.googleapis.com/css2?family=..." rel="stylesheet">
```

## License

MIT

//! ORE Regolith Dashboard - Real-time mining visualization
//! Built with Dioxus for a professional Rust-only frontend

use dioxus::prelude::*;
use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// Asset for the stylesheet
static MAIN_CSS: Asset = asset!("/assets/main.css");

// Constants
const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;
const API_BASE_URL: &str = "";  // Same origin
const POLL_INTERVAL_MS: u32 = 2000;

fn main() {
    dioxus::launch(app);
}

// ═══════════════════════════════════════════════════════════════════════════
// DATA MODELS
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct BoardState {
    pub round_id: u64,
    pub start_slot: u64,
    pub end_slot: u64,
    pub current_slot: u64,
    pub deployed: [u64; 25],
    pub time_remaining_secs: u64,
    pub round_duration_secs: u64,
    pub slots_remaining: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct WinnerInfo {
    pub round_id: u64,
    pub winning_square: u8,
    pub total_pot: u64,
    pub is_motherlode: bool,
    pub timestamp: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct DashboardStats {
    pub total_rounds_today: u64,
    pub total_sol_deployed: f64,
    pub avg_round_time: f64,
    pub motherlode_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ApiResponse {
    pub board: Option<BoardState>,
    pub last_winner: Option<WinnerInfo>,
    pub stats: Option<DashboardStats>,
    pub recent_rounds: Option<Vec<RecentRound>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RecentRound {
    pub round_id: u64,
    pub winning_square: u8,
    pub total_pot: f64,
    pub is_motherlode: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// MAIN APP
// ═══════════════════════════════════════════════════════════════════════════

fn app() -> Element {
    // State
    let mut board = use_signal(BoardState::default);
    let mut last_winner = use_signal(|| None::<WinnerInfo>);
    let mut stats = use_signal(DashboardStats::default);
    let mut recent_rounds = use_signal(|| VecDeque::<RecentRound>::new());
    let mut is_connected = use_signal(|| false);
    let mut show_winner_reveal = use_signal(|| false);
    let mut local_time_remaining = use_signal(|| 0u64);

    // Fetch data from API
    let fetch_data = move || async move {
        match Request::get(&format!("{}/api/state", API_BASE_URL))
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(data) = response.json::<ApiResponse>().await {
                    if let Some(new_board) = data.board {
                        // Check if round changed (new winner)
                        let current_round = board.read().round_id;
                        if current_round > 0 && new_board.round_id != current_round {
                            show_winner_reveal.set(true);
                        }
                        local_time_remaining.set(new_board.time_remaining_secs);
                        board.set(new_board);
                    }
                    if let Some(winner) = data.last_winner {
                        last_winner.set(Some(winner));
                    }
                    if let Some(new_stats) = data.stats {
                        stats.set(new_stats);
                    }
                    if let Some(rounds) = data.recent_rounds {
                        let mut deque = VecDeque::new();
                        for r in rounds.into_iter().take(10) {
                            deque.push_back(r);
                        }
                        recent_rounds.set(deque);
                    }
                    is_connected.set(true);
                }
            }
            Err(_) => {
                is_connected.set(false);
            }
        }
    };

    // Poll for data
    use_future(move || async move {
        loop {
            fetch_data().await;
            TimeoutFuture::new(POLL_INTERVAL_MS).await;
        }
    });

    // Local countdown timer (updates every second)
    use_future(move || async move {
        loop {
            TimeoutFuture::new(1000).await;
            let current = *local_time_remaining.read();
            if current > 0 {
                local_time_remaining.set(current - 1);
            }
        }
    });

    // Auto-hide winner reveal
    use_effect(move || {
        if *show_winner_reveal.read() {
            spawn(async move {
                TimeoutFuture::new(5000).await;
                show_winner_reveal.set(false);
            });
        }
    });

    let board_data = board.read();
    let total_deployed: u64 = board_data.deployed.iter().sum();
    let active_squares = board_data.deployed.iter().filter(|&&d| d > 0).count();
    let time_remaining = *local_time_remaining.read();
    let progress = if board_data.round_duration_secs > 0 {
        ((board_data.round_duration_secs - time_remaining) as f64 / board_data.round_duration_secs as f64 * 100.0) as u32
    } else {
        0
    };

    rsx! {
        Stylesheet { href: MAIN_CSS }
        
        div { class: "dashboard",
            // Header
            Header {
                is_connected: *is_connected.read(),
                round_id: board_data.round_id,
            }
            
            // Main content
            div { class: "main-content",
                // Left side - Grid
                div { class: "grid-section",
                    div { class: "section-header",
                        h2 { class: "section-title",
                            "⛏️ Regolith Grid"
                        }
                        div { class: "round-info",
                            span { class: "round-badge",
                                "Round #{board_data.round_id}"
                            }
                        }
                    }
                    
                    // Winner reveal overlay
                    if *show_winner_reveal.read() {
                        if let Some(winner) = last_winner.read().as_ref() {
                            WinnerReveal {
                                winner: winner.clone(),
                            }
                        }
                    }
                    
                    div { class: "grid-container",
                        // The 5x5 grid
                        RegolithGrid {
                            deployed: board_data.deployed,
                            total_deployed: total_deployed,
                            winning_square: last_winner.read().as_ref().map(|w| w.winning_square),
                            current_round: board_data.round_id,
                            winner_round: last_winner.read().as_ref().map(|w| w.round_id),
                        }
                        
                        // Timer
                        Timer {
                            time_remaining: time_remaining,
                            round_duration: board_data.round_duration_secs,
                            slots_remaining: board_data.slots_remaining,
                            progress: progress,
                        }
                    }
                }
                
                // Right side - Stats
                div { class: "sidebar",
                    // Round stats
                    StatsCard {
                        title: "📊 Round Stats",
                        children: rsx! {
                            div { class: "stats-grid",
                                StatItem {
                                    value: format!("{:.4}", total_deployed as f64 / LAMPORTS_PER_SOL),
                                    label: "Total SOL",
                                }
                                StatItem {
                                    value: format!("{}", active_squares),
                                    label: "Active Squares",
                                }
                                StatItem {
                                    value: format!("{}", board_data.slots_remaining),
                                    label: "Slots Left",
                                }
                                StatItem {
                                    value: format!("{}", board_data.current_slot),
                                    label: "Current Slot",
                                }
                            }
                        }
                    }
                    
                    // Session stats
                    StatsCard {
                        title: "🏆 Session Stats",
                        children: rsx! {
                            div { class: "stats-grid",
                                StatItem {
                                    value: format!("{}", stats.read().total_rounds_today),
                                    label: "Rounds Today",
                                }
                                StatItem {
                                    value: format!("{:.2}", stats.read().total_sol_deployed),
                                    label: "SOL Deployed",
                                }
                                StatItem {
                                    value: format!("{:.1}s", stats.read().avg_round_time),
                                    label: "Avg Round",
                                }
                                StatItem {
                                    value: format!("{}", stats.read().motherlode_count),
                                    label: "Motherlodes",
                                }
                            }
                        }
                    }
                    
                    // Recent rounds
                    StatsCard {
                        title: "📜 Recent Rounds",
                        children: rsx! {
                            div { class: "recent-rounds",
                                for round in recent_rounds.read().iter() {
                                    div { class: "round-item",
                                        key: "{round.round_id}",
                                        div { class: "round-item-left",
                                            span { class: "round-number", "#{round.round_id}" }
                                            span { class: "round-winner-square", "Square {}", round.winning_square + 1 }
                                            if round.is_motherlode {
                                                span { class: "motherlode-badge", "MOTHERLODE" }
                                            }
                                        }
                                        div { class: "round-item-right",
                                            "{round.total_pot:.4} SOL"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// COMPONENTS
// ═══════════════════════════════════════════════════════════════════════════

#[component]
fn Header(is_connected: bool, round_id: u64) -> Element {
    rsx! {
        header { class: "header",
            div { class: "header-left",
                div { class: "logo", "⛏" }
                div { class: "header-title",
                    h1 { "ORE Regolith" }
                    p { "Real-time Mining Dashboard" }
                }
            }
            div { class: "header-right",
                div { class: "connection-status",
                    span { 
                        class: if is_connected { "status-dot connected" } else { "status-dot disconnected" },
                    }
                    span { 
                        if is_connected { "Connected" } else { "Disconnected" }
                    }
                }
            }
        }
    }
}

#[component]
fn RegolithGrid(
    deployed: [u64; 25],
    total_deployed: u64,
    winning_square: Option<u8>,
    current_round: u64,
    winner_round: Option<u64>,
) -> Element {
    let max_deploy = deployed.iter().max().copied().unwrap_or(1);
    
    rsx! {
        div { class: "regolith-grid",
            for (idx, &amount) in deployed.iter().enumerate() {
                {
                    let square_num = idx + 1; // 1-indexed display
                    let is_winner = winning_square.map(|w| w as usize == idx).unwrap_or(false) 
                        && winner_round == Some(current_round.saturating_sub(1));
                    let has_deploys = amount > 0;
                    let heat_level = if max_deploy > 0 {
                        ((amount as f64 / max_deploy as f64) * 5.0).ceil() as u32
                    } else {
                        0
                    };
                    let percentage = if total_deployed > 0 {
                        (amount as f64 / total_deployed as f64) * 100.0
                    } else {
                        0.0
                    };
                    
                    let cell_class = format!(
                        "grid-cell {} {} {}",
                        if is_winner { "winner" } else { "" },
                        if has_deploys { format!("has-deploys heat-{}", heat_level) } else { "".to_string() },
                        ""
                    );
                    
                    rsx! {
                        div {
                            key: "{idx}",
                            class: "{cell_class}",
                            span { class: "cell-number", "#{square_num}" }
                            if amount > 0 {
                                span { class: "cell-amount", 
                                    "{format!(\"{:.3}\", amount as f64 / LAMPORTS_PER_SOL)}"
                                }
                                span { class: "cell-percentage", 
                                    "{percentage:.1}%"
                                }
                            } else {
                                span { class: "cell-amount zero", "—" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Timer(time_remaining: u64, round_duration: u64, slots_remaining: u64, progress: u32) -> Element {
    let minutes = time_remaining / 60;
    let seconds = time_remaining % 60;
    let is_urgent = time_remaining < 10;
    
    rsx! {
        div { class: "timer-container",
            span { class: "timer-label", "Round Ends In" }
            span { 
                class: if is_urgent { "timer-value urgent" } else { "timer-value" },
                "{minutes:02}:{seconds:02}"
            }
            div { class: "timer-progress",
                div { 
                    class: "timer-progress-bar",
                    style: "width: {progress}%",
                }
            }
            span { class: "timer-slots", 
                "{slots_remaining} slots remaining"
            }
        }
    }
}

#[component]
fn WinnerReveal(winner: WinnerInfo) -> Element {
    rsx! {
        div { class: "winner-reveal",
            div { class: "winner-title",
                "🎉 Round {winner.round_id} Winner!"
                if winner.is_motherlode {
                    span { class: "motherlode-badge", "MOTHERLODE 🎰" }
                }
            }
            div { class: "winner-square", 
                "Square #{}", winner.winning_square + 1
            }
            div { class: "winner-details",
                div { class: "winner-detail",
                    span { class: "winner-detail-value", 
                        "{format!(\"{:.4}\", winner.total_pot as f64 / LAMPORTS_PER_SOL)} SOL"
                    }
                    span { "Total Pot" }
                }
            }
        }
    }
}

#[component]
fn StatsCard(title: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "stats-card",
            h3 { class: "stats-card-title", "{title}" }
            {children}
        }
    }
}

#[component]
fn StatItem(value: String, label: &'static str) -> Element {
    rsx! {
        div { class: "stat-item",
            span { class: "stat-value", "{value}" }
            span { class: "stat-label", "{label}" }
        }
    }
}

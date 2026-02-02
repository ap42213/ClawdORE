use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("Solana client error: {0}")]
    SolanaClient(#[from] solana_client::client_error::ClientError),

    #[error("Solana program error: {0}")]
    SolanaProgram(#[from] solana_program::program_error::ProgramError),

    #[error("Anchor error: {0}")]
    Anchor(String),

    #[error("HTTP request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Insufficient balance: {0}")]
    InsufficientBalance(String),

    #[error("Mining error: {0}")]
    Mining(String),

    #[error("Betting error: {0}")]
    Betting(String),

    #[error("Analytics error: {0}")]
    Analytics(String),

    #[error("Strategy error: {0}")]
    Strategy(String),

    #[error("Entropy error: {0}")]
    Entropy(String),

    #[error("Ore-mint error: {0}")]
    OreMint(String),

    #[error("RPC timeout: {0}")]
    RpcTimeout(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, BotError>;

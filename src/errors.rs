use thiserror::Error;

/// Structured error type for the Tokocrypto CLI.
/// Maps to a stable `error` category in JSON error envelopes.
#[derive(Debug, Error)]
pub enum TokocryptoError {
    #[error("API error ({code}): {message}")]
    Api { code: i64, message: String },

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Rate limited: {0}")]
    RateLimit(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl TokocryptoError {
    /// Returns the stable error category string for JSON envelopes.
    pub fn category(&self) -> &'static str {
        match self {
            TokocryptoError::Api { .. } => "api",
            TokocryptoError::Auth(_) => "auth",
            TokocryptoError::Network(_) => "network",
            TokocryptoError::Validation(_) => "validation",
            TokocryptoError::RateLimit(_) => "rate_limit",
            TokocryptoError::Config(_) => "config",
            TokocryptoError::Io(_) => "io",
            TokocryptoError::Parse(_) => "parse",
            TokocryptoError::WebSocket(_) => "websocket",
            TokocryptoError::Internal(_) => "internal",
        }
    }

    /// Whether this error is retryable.
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            TokocryptoError::Network(_)
                | TokocryptoError::RateLimit(_)
                | TokocryptoError::WebSocket(_)
        )
    }

    /// Format this error as a JSON error envelope.
    pub fn to_json_envelope(&self) -> serde_json::Value {
        serde_json::json!({
            "error": true,
            "error_type": self.category(),
            "message": self.to_string(),
            "retryable": self.retryable(),
        })
    }
}

impl From<reqwest::Error> for TokocryptoError {
    fn from(e: reqwest::Error) -> Self {
        TokocryptoError::Network(e.to_string())
    }
}

impl From<serde_json::Error> for TokocryptoError {
    fn from(e: serde_json::Error) -> Self {
        TokocryptoError::Parse(e.to_string())
    }
}

impl From<url::ParseError> for TokocryptoError {
    fn from(e: url::ParseError) -> Self {
        TokocryptoError::Parse(e.to_string())
    }
}

impl From<anyhow::Error> for TokocryptoError {
    fn from(e: anyhow::Error) -> Self {
        TokocryptoError::Api {
            code: -1,
            message: e.to_string(),
        }
    }
}

/// Display for user-facing error output (non-JSON mode).
impl TokocryptoError {
    pub fn to_pretty_string(&self) -> String {
        use colored::Colorize;
        format!("{} {}", "Error:".red().bold(), self)
    }

    pub fn print_pretty(&self) {
        eprintln!("{}", self.to_pretty_string());
    }
}

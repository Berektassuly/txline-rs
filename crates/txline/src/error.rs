//! Error type scaffolding.

/// SDK result type.
pub type Result<T> = std::result::Result<T, TxlineError>;

/// Placeholder SDK error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxlineError {
    message: String,
}

impl TxlineError {
    /// Create a scaffold error with a message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TxlineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TxlineError {}

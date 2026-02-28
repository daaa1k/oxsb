use thiserror::Error;

/// Unified error type for all oxsb operations.
#[derive(Debug, Error)]
pub enum OxsbError {
    /// Configuration file was not found at the specified path.
    #[error("Config not found: {path}")]
    ConfigNotFound { path: String },

    /// Failed to parse the YAML configuration.
    #[error("Failed to parse config: {0}")]
    ConfigParse(#[from] serde_yml::Error),

    /// An underlying I/O error occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A path marked as required does not exist on the filesystem.
    #[error("Required path does not exist: {path}")]
    RequiredPathMissing { path: String },

    /// The requested backend is not available on the current platform.
    #[error("Backend '{backend}' is not available on this platform")]
    BackendUnavailable { backend: String },

    /// Failed to exec the target command.
    #[error("Exec failed: {0}")]
    ExecFailed(String),

    /// An unknown variable was referenced in a path expression.
    #[error("Unknown variable '{var}' in path expansion")]
    UnknownVariable { var: String },
}

/// Convenience alias for `Result<T, OxsbError>`.
pub type Result<T> = std::result::Result<T, OxsbError>;

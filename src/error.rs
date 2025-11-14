use thiserror::Error;

/// Custom error type for bloaty-metafile operations
#[derive(Error, Debug)]
pub enum BloatyError {
    /// Error reading a file from the filesystem
    #[error("Failed to read file: {path}")]
    FileRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Error parsing CSV data
    #[error("Failed to parse CSV")]
    CsvParse(#[from] csv::Error),

    /// Error serializing to JSON
    #[error("Failed to serialize JSON")]
    JsonSerialize(#[from] serde_json::Error),

    /// Error loading Cargo.lock file
    #[error("Failed to load Cargo.lock: {path}")]
    LockfileLoad {
        path: String,
        #[source]
        source: cargo_lock::Error,
    },
}

/// Result type alias for bloaty-metafile operations
pub type Result<T> = std::result::Result<T, BloatyError>;

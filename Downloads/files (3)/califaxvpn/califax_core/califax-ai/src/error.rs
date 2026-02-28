//! Error types for the califax-ai crate.

use thiserror::Error;

/// Errors that can occur during AI model operations.
#[derive(Debug, Error)]
pub enum AiError {
    /// The ONNX model file could not be loaded from disk.
    #[error("failed to load model from '{path}': {reason}")]
    ModelLoadFailed {
        path: String,
        reason: String,
    },

    /// An error occurred during model inference.
    #[error("inference failed: {0}")]
    InferenceFailed(String),

    /// The input data is invalid (wrong shape, NaN values, etc.).
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Attempted to run inference before a model was loaded.
    #[error("model not loaded — call load() or new() first")]
    ModelNotLoaded,
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, AiError>;

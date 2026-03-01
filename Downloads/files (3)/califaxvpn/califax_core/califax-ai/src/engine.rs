//! Generic ONNX Runtime inference engine.

use std::path::Path;

use ort::session::Session;
use tracing::info;

use crate::error::{AiError, Result};

/// A thin wrapper around an `ort::Session` that loads an ONNX model and
/// runs single-batch inference.
pub struct OnnxEngine {
    session: Session,
}

impl OnnxEngine {
    /// Load an ONNX model from the given file path.
    pub fn load<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        let path = model_path.as_ref();
        info!(path = %path.display(), "loading ONNX model");

        let session = Session::builder()
            .map_err(|e| AiError::ModelLoadFailed {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?
            .commit_from_file(path)
            .map_err(|e| AiError::ModelLoadFailed {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;

        Ok(Self { session })
    }

    /// Run inference on a flat `f32` slice, reshaping it according to
    /// `input_shape` (e.g. `[1, 12]` for a single sample with 12 features).
    ///
    /// Returns the first output tensor flattened into a `Vec<f32>`.
    pub fn predict(&mut self, input: &[f32], input_shape: &[usize]) -> Result<Vec<f32>> {
        let expected_len: usize = input_shape.iter().product();
        if input.len() != expected_len {
            return Err(AiError::InvalidInput(format!(
                "expected {} elements for shape {:?}, got {}",
                expected_len, input_shape, input.len(),
            )));
        }

        // Use (shape, data) tuple to create the Value — avoids ndarray version mismatch.
        let shape: Vec<i64> = input_shape.iter().map(|&s| s as i64).collect();
        let input_value = ort::value::Value::from_array((shape.as_slice(), input.to_vec()))
            .map_err(|e| AiError::InferenceFailed(e.to_string()))?;

        let outputs = self
            .session
            .run(ort::inputs![input_value])
            .map_err(|e| AiError::InferenceFailed(e.to_string()))?;

        // Extract the first output tensor.
        let (_name, output_tensor) = outputs
            .iter()
            .next()
            .ok_or_else(|| AiError::InferenceFailed("model produced no outputs".into()))?;

        let (_shape, data) = output_tensor
            .try_extract_tensor::<f32>()
            .map_err(|e| AiError::InferenceFailed(e.to_string()))?;

        Ok(data.to_vec())
    }
}

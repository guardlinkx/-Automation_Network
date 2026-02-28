//! Generic ONNX Runtime inference engine.

use std::path::Path;

use ndarray::Array2;
use ort::{Session, Value};
use tracing::info;

use crate::error::{AiError, Result};

/// A thin wrapper around an `ort::Session` that loads an ONNX model and
/// runs single-batch inference.
pub struct OnnxEngine {
    session: Session,
}

impl OnnxEngine {
    /// Load an ONNX model from the given file path.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::ModelLoadFailed`] if the file cannot be read or the
    /// ONNX runtime rejects the model.
    pub fn load<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        let path = model_path.as_ref();
        info!(path = %path.display(), "loading ONNX model");

        let session = Session::builder()
            .and_then(|b| b.with_model_from_file(path))
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
    ///
    /// # Errors
    ///
    /// - [`AiError::InvalidInput`] if the slice length does not match the
    ///   product of `input_shape`.
    /// - [`AiError::InferenceFailed`] on any ONNX runtime error.
    pub fn predict(&self, input: &[f32], input_shape: &[usize]) -> Result<Vec<f32>> {
        let expected_len: usize = input_shape.iter().product();
        if input.len() != expected_len {
            return Err(AiError::InvalidInput(format!(
                "expected {} elements for shape {:?}, got {}",
                expected_len, input_shape, input.len(),
            )));
        }

        // Build an ndarray and wrap it in an ort::Value.
        let rows = input_shape[0];
        let cols = if input_shape.len() > 1 { input_shape[1] } else { 1 };
        let array = Array2::from_shape_vec((rows, cols), input.to_vec())
            .map_err(|e| AiError::InvalidInput(e.to_string()))?;

        let input_value = Value::from_array(array)
            .map_err(|e| AiError::InferenceFailed(e.to_string()))?;

        let outputs = self
            .session
            .run(ort::inputs![input_value].map_err(|e| AiError::InferenceFailed(e.to_string()))?)
            .map_err(|e| AiError::InferenceFailed(e.to_string()))?;

        // Extract the first output tensor.
        let output_tensor = outputs
            .get(0)
            .ok_or_else(|| AiError::InferenceFailed("model produced no outputs".into()))?;

        let output_array = output_tensor
            .try_extract_tensor::<f32>()
            .map_err(|e| AiError::InferenceFailed(e.to_string()))?;

        Ok(output_array.as_slice().unwrap_or_default().to_vec())
    }
}

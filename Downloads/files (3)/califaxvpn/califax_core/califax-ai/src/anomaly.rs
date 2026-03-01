//! Anomaly scoring module.
//!
//! Wraps the Isolation Forest model that produces continuous anomaly scores
//! for network traffic metrics.

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::engine::OnnxEngine;
use crate::error::{AiError, Result};

/// Number of input features expected by the anomaly-scoring model.
const NUM_FEATURES: usize = 8;

/// Anomaly-detection threshold. Scores below this value are flagged.
/// Isolation Forest decision_function returns negative values for anomalies;
/// the ONNX export may output the raw score or the label.
const ANOMALY_THRESHOLD: f32 = -0.1;

/// Raw input features describing a traffic session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyInput {
    pub bytes_per_second: f32,
    pub packets_per_second: f32,
    pub unique_destinations: f32,
    pub avg_packet_size: f32,
    pub connection_duration: f32,
    pub reconnect_frequency: f32,
    pub dns_query_rate: f32,
    pub failed_connections: f32,
}

impl AnomalyInput {
    /// Flatten the input into a feature vector.
    fn to_features(&self) -> [f32; NUM_FEATURES] {
        [
            self.bytes_per_second,
            self.packets_per_second,
            self.unique_destinations,
            self.avg_packet_size,
            self.connection_duration,
            self.reconnect_frequency,
            self.dns_query_rate,
            self.failed_connections,
        ]
    }
}

/// Result of anomaly scoring for a single session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyResult {
    /// Raw anomaly score from the model. Lower (more negative) = more
    /// anomalous.
    pub score: f32,
    /// Whether the session is classified as anomalous.
    pub is_anomalous: bool,
}

/// ONNX-backed anomaly scorer.
pub struct AnomalyScorer {
    engine: OnnxEngine,
}

impl AnomalyScorer {
    /// Load the anomaly-scoring ONNX model from `model_path`.
    pub fn new<P: AsRef<std::path::Path>>(model_path: P) -> Result<Self> {
        let engine = OnnxEngine::load(model_path)?;
        Ok(Self { engine })
    }

    /// Score a single traffic session.
    pub fn score(&mut self, input: &AnomalyInput) -> Result<AnomalyResult> {
        let features = input.to_features();

        if features.iter().any(|v| !v.is_finite()) {
            return Err(AiError::InvalidInput(
                "input contains NaN or Inf values".into(),
            ));
        }

        let output = self.engine.predict(&features, &[1, NUM_FEATURES])?;

        // The sklearn ONNX export for IsolationForest may produce:
        //   output[0]: predicted label (1 or -1)
        //   output[1]: decision_function score
        // We try to extract the score; fall back to the raw first value.
        let score = if output.len() >= 2 {
            output[1]
        } else {
            output.first().copied().unwrap_or(0.0)
        };

        let is_anomalous = score < ANOMALY_THRESHOLD;

        debug!(score, is_anomalous, "anomaly scoring complete");

        Ok(AnomalyResult {
            score,
            is_anomalous,
        })
    }
}

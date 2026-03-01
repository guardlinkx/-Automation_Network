//! Route optimization module.
//!
//! Wraps the LightGBM regressor that predicts end-to-end latency for a given
//! VPN route configuration, enabling intelligent server selection.

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::engine::OnnxEngine;
use crate::error::{AiError, Result};

/// Number of input features expected by the routing model.
const NUM_FEATURES: usize = 8;

/// Input features describing a candidate VPN route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInput {
    /// Encoded source region (0..7).
    pub source_region: f32,
    /// Encoded destination region (0..7).
    pub dest_region: f32,
    /// Current server load factor (0.0 -- 1.0).
    pub current_load: f32,
    /// Available bandwidth in Mbps.
    pub bandwidth_mbps: f32,
    /// Number of network hops.
    pub hop_count: f32,
    /// Hour of day in UTC (0.0 -- 24.0).
    pub time_of_day: f32,
    /// Current packet-loss rate (0.0 -- 1.0).
    pub packet_loss_rate: f32,
    /// Observed jitter in milliseconds.
    pub jitter_ms: f32,
}

impl RouteInput {
    /// Flatten the input into a feature vector.
    fn to_features(&self) -> [f32; NUM_FEATURES] {
        [
            self.source_region,
            self.dest_region,
            self.current_load,
            self.bandwidth_mbps,
            self.hop_count,
            self.time_of_day,
            self.packet_loss_rate,
            self.jitter_ms,
        ]
    }
}

/// ONNX-backed route optimizer.
pub struct RouteOptimizer {
    engine: OnnxEngine,
}

impl RouteOptimizer {
    /// Load the route-optimization ONNX model from `model_path`.
    pub fn new<P: AsRef<std::path::Path>>(model_path: P) -> Result<Self> {
        let engine = OnnxEngine::load(model_path)?;
        Ok(Self { engine })
    }

    /// Predict the expected latency (in milliseconds) for a single route.
    pub fn predict_latency(&mut self, input: &RouteInput) -> Result<f32> {
        let features = input.to_features();

        if features.iter().any(|v| !v.is_finite()) {
            return Err(AiError::InvalidInput(
                "input contains NaN or Inf values".into(),
            ));
        }

        let output = self.engine.predict(&features, &[1, NUM_FEATURES])?;

        let latency = output
            .first()
            .copied()
            .ok_or_else(|| AiError::InferenceFailed("model returned empty output".into()))?;

        debug!(latency_ms = latency, "latency prediction complete");

        Ok(latency)
    }

    /// Given a set of candidate routes, return the index of the one with the
    /// lowest predicted latency.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::InvalidInput`] if `candidates` is empty, or
    /// propagates any inference error.
    pub fn recommend_route(&mut self, candidates: &[RouteInput]) -> Result<usize> {
        if candidates.is_empty() {
            return Err(AiError::InvalidInput(
                "candidates list must not be empty".into(),
            ));
        }

        let mut best_idx: usize = 0;
        let mut best_latency = f32::MAX;

        for (i, candidate) in candidates.iter().enumerate() {
            let latency = self.predict_latency(candidate)?;
            if latency < best_latency {
                best_latency = latency;
                best_idx = i;
            }
        }

        debug!(
            best_idx,
            best_latency_ms = best_latency,
            total_candidates = candidates.len(),
            "route recommendation complete"
        );

        Ok(best_idx)
    }
}

//! Threat detection module.
//!
//! Wraps the XGBoost-trained binary classifier that distinguishes normal
//! network flows from malicious ones (DDoS, port-scan, exfiltration, etc.).

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::engine::OnnxEngine;
use crate::error::{AiError, Result};

/// Number of input features expected by the threat-detection model.
const NUM_FEATURES: usize = 12;

/// Raw input features for a single network flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatInput {
    pub packet_size: f32,
    pub flow_duration: f32,
    pub packet_rate: f32,
    pub byte_rate: f32,
    pub protocol_type: f32,
    pub port_entropy: f32,
    pub payload_entropy: f32,
    pub flag_syn: f32,
    pub flag_ack: f32,
    pub flag_rst: f32,
    pub flow_iat_mean: f32,
    pub flow_iat_std: f32,
}

impl ThreatInput {
    /// Flatten the input into a feature vector in the order expected by the
    /// ONNX model.
    fn to_features(&self) -> [f32; NUM_FEATURES] {
        [
            self.packet_size,
            self.flow_duration,
            self.packet_rate,
            self.byte_rate,
            self.protocol_type,
            self.port_entropy,
            self.payload_entropy,
            self.flag_syn,
            self.flag_ack,
            self.flag_rst,
            self.flow_iat_mean,
            self.flow_iat_std,
        ]
    }
}

/// Result of threat analysis for a single flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatResult {
    /// Whether the flow is classified as a threat.
    pub is_threat: bool,
    /// Model confidence in the prediction (0.0 -- 1.0).
    pub confidence: f32,
    /// Heuristic threat-type label when the flow is flagged as a threat.
    pub threat_type: Option<String>,
}

/// ONNX-backed threat detector.
pub struct ThreatDetector {
    engine: OnnxEngine,
}

impl ThreatDetector {
    /// Load the threat-detection ONNX model from `model_path`.
    pub fn new<P: AsRef<std::path::Path>>(model_path: P) -> Result<Self> {
        let engine = OnnxEngine::load(model_path)?;
        Ok(Self { engine })
    }

    /// Analyse a single network flow and return a [`ThreatResult`].
    pub fn analyze(&mut self, input: &ThreatInput) -> Result<ThreatResult> {
        let features = input.to_features();

        // Validate -- reject NaN / Inf values.
        if features.iter().any(|v| !v.is_finite()) {
            return Err(AiError::InvalidInput(
                "input contains NaN or Inf values".into(),
            ));
        }

        let output = self.engine.predict(&features, &[1, NUM_FEATURES])?;

        // The XGBoost ONNX export typically outputs two values per sample:
        // [prob_class_0, prob_class_1]. If we only get one value it is the
        // raw logistic score.
        let (confidence, is_threat) = if output.len() >= 2 {
            let prob_threat = output[1];
            (prob_threat, prob_threat >= 0.5)
        } else {
            let score = output.first().copied().unwrap_or(0.0);
            (score, score >= 0.5)
        };

        let threat_type = if is_threat {
            Some(Self::classify_threat_type(input))
        } else {
            None
        };

        debug!(
            is_threat,
            confidence,
            threat_type = ?threat_type,
            "threat analysis complete"
        );

        Ok(ThreatResult {
            is_threat,
            confidence,
            threat_type,
        })
    }

    /// Simple heuristic to attach a human-readable threat category based on
    /// the dominant feature anomaly.
    fn classify_threat_type(input: &ThreatInput) -> String {
        if input.packet_rate > 1000.0 {
            "DDoS".to_string()
        } else if input.port_entropy > 4.0 {
            "PortScan".to_string()
        } else if input.payload_entropy > 6.5 {
            "DataExfiltration".to_string()
        } else {
            "Unknown".to_string()
        }
    }
}

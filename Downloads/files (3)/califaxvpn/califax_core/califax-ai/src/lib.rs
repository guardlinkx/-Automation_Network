//! # califax-ai
//!
//! Machine-learning inference crate for the Califax VPN project.
//!
//! Provides ONNX-backed models for:
//! - **Threat detection** -- binary classification of network flows.
//! - **Anomaly scoring** -- unsupervised anomaly detection on traffic metrics.
//! - **Route optimization** -- latency prediction for intelligent routing.
//!
//! All models are loaded from ONNX files produced by the Python training
//! pipeline (`califaxvpn/ml_pipeline/`).

pub mod anomaly;
pub mod engine;
pub mod error;
pub mod routing;
pub mod threat;

pub use anomaly::{AnomalyInput, AnomalyResult, AnomalyScorer};
pub use engine::OnnxEngine;
pub use error::AiError;
pub use routing::{RouteInput, RouteOptimizer};
pub use threat::{ThreatDetector, ThreatInput, ThreatResult};

use crate::error::IdentityError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPolicy {
    pub resource: String,
    pub min_trust_score: i32,
    pub require_zk_proof: bool,
    pub require_canary_alive: bool,
    pub allowed_regions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessContext {
    pub did: String,
    pub trust_score: i32,
    pub device_is_new: bool,
    pub location_is_unusual: bool,
    pub is_off_hours: bool,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessDecision {
    pub allowed: bool,
    pub trust_score: i32,
    pub reasons: Vec<String>,
    pub factors_evaluated: Vec<String>,
}

pub struct ZeroTrustEngine {
    default_min_trust: i32,
}

impl ZeroTrustEngine {
    pub fn new(default_min_trust: i32) -> Self {
        Self { default_min_trust }
    }

    pub fn evaluate(&self, context: &AccessContext, policy: Option<&AccessPolicy>) -> AccessDecision {
        let min_trust = policy.map(|p| p.min_trust_score).unwrap_or(self.default_min_trust);
        let mut score = context.trust_score;
        let mut reasons = Vec::new();
        let mut factors = vec!["identity".to_string()];

        if context.device_is_new {
            score -= 20;
            reasons.push("new_device_penalty".to_string());
            factors.push("device".to_string());
        }
        if context.location_is_unusual {
            score -= 15;
            reasons.push("unusual_location_penalty".to_string());
            factors.push("location".to_string());
        }
        if context.is_off_hours {
            score -= 10;
            reasons.push("off_hours_penalty".to_string());
            factors.push("time".to_string());
        }

        if let Some(policy) = policy {
            if !policy.allowed_regions.is_empty() {
                if let Some(ref region) = context.region {
                    if !policy.allowed_regions.contains(region) {
                        score -= 50;
                        reasons.push("region_not_allowed".to_string());
                    }
                }
                factors.push("region".to_string());
            }
        }

        let allowed = score >= min_trust;
        if !allowed {
            reasons.push(format!("trust_score_{}_below_threshold_{}", score, min_trust));
        }

        AccessDecision { allowed, trust_score: score, reasons, factors_evaluated: factors }
    }
}

impl Default for ZeroTrustEngine {
    fn default() -> Self { Self::new(50) }
}

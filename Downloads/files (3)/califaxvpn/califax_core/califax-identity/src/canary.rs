use crate::error::IdentityError;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CanaryStatus {
    Alive,
    Dead,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryState {
    pub status: CanaryStatus,
    pub last_chirp_epoch: u64,
    pub chirp_interval_secs: u64,
    pub operator: String,
    pub message: String,
}

pub struct WarrantCanary {
    state: RwLock<CanaryState>,
}

impl WarrantCanary {
    pub fn new(operator: &str, chirp_interval_secs: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        Self {
            state: RwLock::new(CanaryState {
                status: CanaryStatus::Alive,
                last_chirp_epoch: now,
                chirp_interval_secs,
                operator: operator.to_string(),
                message: "No warrants, subpoenas, or gag orders received.".to_string(),
            }),
        }
    }

    pub fn chirp(&self, message: Option<&str>) {
        let mut state = self.state.write().unwrap();
        state.last_chirp_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        state.status = CanaryStatus::Alive;
        if let Some(msg) = message { state.message = msg.to_string(); }
    }

    pub fn kill(&self) {
        let mut state = self.state.write().unwrap();
        state.status = CanaryStatus::Dead;
        state.message = "Canary has been silenced.".to_string();
    }

    pub fn check(&self) -> CanaryState {
        let mut state = self.state.write().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        if state.status == CanaryStatus::Alive && now - state.last_chirp_epoch > state.chirp_interval_secs {
            state.status = CanaryStatus::Stale;
        }
        state.clone()
    }

    pub fn is_alive(&self) -> bool {
        self.check().status == CanaryStatus::Alive
    }
}

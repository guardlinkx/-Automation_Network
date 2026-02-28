use crate::error::LocationError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    pub app_pattern: String,
    pub block_gps: bool,
    pub block_wifi_scan: bool,
    pub block_cell_info: bool,
    pub block_bluetooth_scan: bool,
}

pub struct LocationFirewall {
    rules: Vec<FirewallRule>,
    default_block_all: bool,
}

impl LocationFirewall {
    pub fn new(default_block_all: bool) -> Self {
        Self { rules: Vec::new(), default_block_all }
    }

    pub fn add_rule(&mut self, rule: FirewallRule) {
        self.rules.push(rule);
    }

    pub fn should_block_gps(&self, app_id: &str) -> bool {
        for rule in &self.rules {
            if app_matches(&rule.app_pattern, app_id) {
                return rule.block_gps;
            }
        }
        self.default_block_all
    }

    pub fn should_block_wifi(&self, app_id: &str) -> bool {
        for rule in &self.rules {
            if app_matches(&rule.app_pattern, app_id) {
                return rule.block_wifi_scan;
            }
        }
        self.default_block_all
    }

    pub fn should_block_cell(&self, app_id: &str) -> bool {
        for rule in &self.rules {
            if app_matches(&rule.app_pattern, app_id) {
                return rule.block_cell_info;
            }
        }
        self.default_block_all
    }

    pub fn should_block_bluetooth(&self, app_id: &str) -> bool {
        for rule in &self.rules {
            if app_matches(&rule.app_pattern, app_id) {
                return rule.block_bluetooth_scan;
            }
        }
        self.default_block_all
    }

    pub fn check_all(&self, app_id: &str) -> FirewallDecision {
        FirewallDecision {
            app_id: app_id.to_string(),
            gps_blocked: self.should_block_gps(app_id),
            wifi_blocked: self.should_block_wifi(app_id),
            cell_blocked: self.should_block_cell(app_id),
            bluetooth_blocked: self.should_block_bluetooth(app_id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallDecision {
    pub app_id: String,
    pub gps_blocked: bool,
    pub wifi_blocked: bool,
    pub cell_blocked: bool,
    pub bluetooth_blocked: bool,
}

fn app_matches(pattern: &str, app_id: &str) -> bool {
    if pattern == "*" { return true; }
    if pattern.ends_with('*') {
        app_id.starts_with(&pattern[..pattern.len() - 1])
    } else {
        pattern == app_id
    }
}

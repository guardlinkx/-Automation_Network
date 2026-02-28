use crate::circuit::{Circuit, CircuitBuilder, CircuitStatus};
use crate::error::MeshError;
use crate::peer::PeerRegistry;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub enum HealingAction {
    Reshuffle { circuit_id: String },
    RebuildCircuit { circuit_id: String, dead_peer: String, hop_index: usize },
}

pub struct SelfHealingMonitor {
    registry: Arc<PeerRegistry>,
    builder: CircuitBuilder,
    active_circuits: Arc<RwLock<Vec<Circuit>>>,
    peer_timeout: Duration,
}

impl SelfHealingMonitor {
    pub fn new(registry: Arc<PeerRegistry>, builder: CircuitBuilder, peer_timeout: Duration) -> Self {
        Self { registry, builder, active_circuits: Arc::new(RwLock::new(Vec::new())), peer_timeout }
    }

    pub fn monitor_circuit(&self, circuit: Circuit) {
        self.active_circuits.write().unwrap().push(circuit);
    }

    pub fn unmonitor_circuit(&self, circuit_id: &str) {
        self.active_circuits.write().unwrap().retain(|c| c.circuit_id != circuit_id);
    }

    pub fn health_check(&self) -> Vec<HealingAction> {
        let mut actions = Vec::new();
        let mut circuits = self.active_circuits.write().unwrap();
        for circuit in circuits.iter_mut() {
            if circuit.status != CircuitStatus::Active { continue; }
            if circuit.is_expired() {
                info!(circuit_id = %circuit.circuit_id, "Circuit expired");
                circuit.status = CircuitStatus::Reshuffling;
                actions.push(HealingAction::Reshuffle { circuit_id: circuit.circuit_id.clone() });
                continue;
            }
            for hop in &circuit.hops {
                let alive = self.registry.get(&hop.peer_id).map(|p| p.is_alive(self.peer_timeout)).unwrap_or(false);
                if !alive {
                    warn!(circuit_id = %circuit.circuit_id, peer_id = %hop.peer_id, "Dead node detected");
                    circuit.status = CircuitStatus::Reshuffling;
                    actions.push(HealingAction::RebuildCircuit {
                        circuit_id: circuit.circuit_id.clone(),
                        dead_peer: hop.peer_id.clone(),
                        hop_index: hop.hop_index,
                    });
                    break;
                }
            }
        }
        actions
    }

    pub fn heal(&self, actions: &[HealingAction]) -> Vec<Result<Circuit, MeshError>> {
        let circuits = self.active_circuits.read().unwrap();
        actions.iter().filter_map(|action| {
            match action {
                HealingAction::Reshuffle { circuit_id } | HealingAction::RebuildCircuit { circuit_id, .. } => {
                    circuits.iter().find(|c| c.circuit_id == *circuit_id)
                        .map(|old| self.builder.reshuffle(&self.registry, old))
                }
            }
        }).collect()
    }

    pub fn active_circuit_count(&self) -> usize {
        self.active_circuits.read().unwrap().iter().filter(|c| c.status == CircuitStatus::Active).count()
    }
}

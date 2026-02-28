use crate::error::LocationError;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    pub fn to_string(&self) -> String {
        self.0.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(":")
    }

    pub fn random_locally_administered() -> Self {
        let mut rng = rand::thread_rng();
        let mut bytes: [u8; 6] = rng.gen();
        bytes[0] = (bytes[0] | 0x02) & 0xFE; // Set locally administered, clear multicast
        Self(bytes)
    }
}

pub struct WifiMasker {
    enabled: bool,
    rotate_interval_secs: u64,
    current_mac: MacAddress,
}

impl WifiMasker {
    pub fn new(rotate_interval_secs: u64) -> Self {
        Self {
            enabled: true,
            rotate_interval_secs,
            current_mac: MacAddress::random_locally_administered(),
        }
    }

    pub fn rotate_mac(&mut self) -> MacAddress {
        self.current_mac = MacAddress::random_locally_administered();
        self.current_mac.clone()
    }

    pub fn current_mac(&self) -> &MacAddress {
        &self.current_mac
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Generate a fake Wi-Fi scan result to feed to apps
    pub fn generate_fake_scan(&self, count: usize) -> Vec<FakeAccessPoint> {
        let mut rng = rand::thread_rng();
        let ssid_prefixes = ["Starbucks", "ATT-WiFi", "xfinity", "NETGEAR", "linksys", "HomeNet", "Guest"];
        (0..count).map(|_| {
            let prefix = ssid_prefixes[rng.gen_range(0..ssid_prefixes.len())];
            FakeAccessPoint {
                ssid: format!("{}-{:04X}", prefix, rng.gen::<u16>()),
                bssid: MacAddress::random_locally_administered(),
                signal_dbm: rng.gen_range(-85..-30),
                channel: *[1, 6, 11, 36, 40, 44, 48].choose(&mut rng).unwrap(),
                security: "WPA2".to_string(),
            }
        }).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakeAccessPoint {
    pub ssid: String,
    pub bssid: MacAddress,
    pub signal_dbm: i32,
    pub channel: i32,
    pub security: String,
}

use rand::seq::SliceRandom;

use crate::error::LocationError;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoCoordinate {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpoofedLocation {
    pub original: GeoCoordinate,
    pub spoofed: GeoCoordinate,
    pub fuzz_radius_meters: f64,
    pub actual_offset_meters: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpoofPolicy {
    pub target_app: Option<String>,
    pub spoof_coordinate: Option<GeoCoordinate>,
    pub fuzz_radius_meters: f64,
    pub enabled: bool,
}

pub struct GpsSpoofEngine {
    policies: Vec<SpoofPolicy>,
}

impl GpsSpoofEngine {
    pub fn new() -> Self {
        Self { policies: Vec::new() }
    }

    pub fn add_policy(&mut self, policy: SpoofPolicy) {
        self.policies.push(policy);
    }

    pub fn clear_policies(&mut self) {
        self.policies.clear();
    }

    /// Apply location fuzzing: random offset within radius
    pub fn fuzz_location(&self, lat: f64, lon: f64, radius_meters: f64) -> Result<SpoofedLocation, LocationError> {
        if lat < -90.0 || lat > 90.0 || lon < -180.0 || lon > 180.0 {
            return Err(LocationError::InvalidCoordinates { lat, lon });
        }

        let mut rng = rand::thread_rng();
        let angle = rng.gen_range(0.0..2.0 * PI);
        let distance = rng.gen_range(0.0..radius_meters);

        // 1 degree lat ≈ 111,000 meters
        let dlat = (distance * angle.cos()) / 111_000.0;
        let dlon = (distance * angle.sin()) / (111_000.0 * (lat * PI / 180.0).cos());

        Ok(SpoofedLocation {
            original: GeoCoordinate { latitude: lat, longitude: lon },
            spoofed: GeoCoordinate { latitude: lat + dlat, longitude: lon + dlon },
            fuzz_radius_meters: radius_meters,
            actual_offset_meters: distance,
        })
    }

    /// Spoof to a specific coordinate
    pub fn spoof_to(&self, original_lat: f64, original_lon: f64, target: &GeoCoordinate) -> SpoofedLocation {
        SpoofedLocation {
            original: GeoCoordinate { latitude: original_lat, longitude: original_lon },
            spoofed: target.clone(),
            fuzz_radius_meters: 0.0,
            actual_offset_meters: haversine_distance(original_lat, original_lon, target.latitude, target.longitude),
        }
    }

    /// Get the effective spoofed location for an app
    pub fn get_spoofed_location(&self, app_id: &str, real_lat: f64, real_lon: f64) -> Result<SpoofedLocation, LocationError> {
        for policy in &self.policies {
            if !policy.enabled { continue; }
            let matches = policy.target_app.as_ref().map(|a| a == app_id || a == "*").unwrap_or(true);
            if matches {
                if let Some(ref target) = policy.spoof_coordinate {
                    return Ok(self.spoof_to(real_lat, real_lon, target));
                } else {
                    return self.fuzz_location(real_lat, real_lon, policy.fuzz_radius_meters);
                }
            }
        }
        // No matching policy - return real location
        Ok(SpoofedLocation {
            original: GeoCoordinate { latitude: real_lat, longitude: real_lon },
            spoofed: GeoCoordinate { latitude: real_lat, longitude: real_lon },
            fuzz_radius_meters: 0.0,
            actual_offset_meters: 0.0,
        })
    }
}

impl Default for GpsSpoofEngine {
    fn default() -> Self { Self::new() }
}

fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6_371_000.0; // Earth radius in meters
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2) + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    r * c
}

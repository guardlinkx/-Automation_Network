"""Location Privacy Layer — GPS spoofing policies, location fuzzing, Wi-Fi masking."""

from flask import Blueprint, request, jsonify
from models import db, LocationPolicy, ThreatEvent
from datetime import datetime, timezone

location_bp = Blueprint("location", __name__, url_prefix="/api/location")

# Try importing Rust location engine
try:
    from califax_core import location as location_engine
    LOCATION_AVAILABLE = True
except ImportError:
    LOCATION_AVAILABLE = False


@location_bp.route("/status", methods=["GET"])
def location_status():
    """Return location privacy engine status."""
    return jsonify({
        "location_engine_available": LOCATION_AVAILABLE,
        "features": {
            "gps_spoofing": LOCATION_AVAILABLE,
            "location_fuzzing": True,  # Available via Python fallback
            "wifi_masking": LOCATION_AVAILABLE,
            "cell_tower_masking": LOCATION_AVAILABLE,
        },
    })


@location_bp.route("/policies", methods=["GET"])
def list_policies():
    """List location policies for a device."""
    device_id = request.args.get("device_id")
    if not device_id:
        return jsonify({"error": "device_id query param required"}), 400
    policies = LocationPolicy.query.filter_by(device_id=device_id, is_active=True).all()
    return jsonify({"policies": [p.to_dict() for p in policies]})


@location_bp.route("/policies", methods=["POST"])
def create_policy():
    """Create a new location privacy policy."""
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    required = ["device_id", "policy_type"]
    missing = [f for f in required if f not in data]
    if missing:
        return jsonify({"error": f"Missing fields: {missing}"}), 400

    policy_type = data["policy_type"]
    valid_types = ["gps_spoof", "location_fuzz", "wifi_mask", "cell_mask", "ble_mask", "full_cloak"]
    if policy_type not in valid_types:
        return jsonify({"error": f"Invalid policy_type. Must be one of: {valid_types}"}), 400

    policy = LocationPolicy(
        device_id=data["device_id"],
        policy_type=policy_type,
        spoof_latitude=data.get("spoof_latitude"),
        spoof_longitude=data.get("spoof_longitude"),
        fuzz_radius_meters=data.get("fuzz_radius_meters", 1000),
        target_apps=data.get("target_apps", ""),
        config_json=data.get("config", {}),
    )
    db.session.add(policy)
    db.session.commit()
    return jsonify({"policy": policy.to_dict()}), 201


@location_bp.route("/policies/<int:policy_id>", methods=["PUT"])
def update_policy(policy_id):
    """Update a location policy."""
    policy = LocationPolicy.query.get_or_404(policy_id)
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    if "spoof_latitude" in data:
        policy.spoof_latitude = data["spoof_latitude"]
    if "spoof_longitude" in data:
        policy.spoof_longitude = data["spoof_longitude"]
    if "fuzz_radius_meters" in data:
        policy.fuzz_radius_meters = data["fuzz_radius_meters"]
    if "target_apps" in data:
        policy.target_apps = data["target_apps"]
    if "config" in data:
        policy.config_json = data["config"]
    if "is_active" in data:
        policy.is_active = data["is_active"]

    db.session.commit()
    return jsonify({"policy": policy.to_dict()})


@location_bp.route("/policies/<int:policy_id>", methods=["DELETE"])
def delete_policy(policy_id):
    """Delete a location policy."""
    policy = LocationPolicy.query.get_or_404(policy_id)
    policy.is_active = False
    db.session.commit()
    return jsonify({"status": "deleted", "policy_id": policy_id})


@location_bp.route("/spoof/preview", methods=["POST"])
def spoof_preview():
    """Preview what a GPS spoof would look like for a given configuration."""
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    lat = data.get("latitude", 0.0)
    lon = data.get("longitude", 0.0)
    fuzz = data.get("fuzz_radius_meters", 1000)

    import math, random
    # Generate a fuzzed position within the radius
    angle = random.uniform(0, 2 * math.pi)
    distance = random.uniform(0, fuzz)
    # Approximate: 1 degree lat ≈ 111km
    dlat = (distance * math.cos(angle)) / 111000
    dlon = (distance * math.sin(angle)) / (111000 * math.cos(math.radians(lat)))

    return jsonify({
        "original": {"latitude": lat, "longitude": lon},
        "spoofed": {"latitude": lat + dlat, "longitude": lon + dlon},
        "fuzz_radius_meters": fuzz,
        "actual_offset_meters": round(distance, 2),
    })


@location_bp.route("/threats", methods=["GET"])
def list_threats():
    """List recent threat events."""
    limit = request.args.get("limit", 50, type=int)
    events = ThreatEvent.query.order_by(ThreatEvent.detected_at.desc()).limit(limit).all()
    return jsonify({"events": [e.to_dict() for e in events]})


@location_bp.route("/threats", methods=["POST"])
def report_threat():
    """Report a new threat event."""
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    event = ThreatEvent(
        event_type=data.get("event_type", "unknown"),
        severity=data.get("severity", "medium"),
        source_ip=data.get("source_ip", ""),
        description=data.get("description", ""),
        metadata_json=data.get("metadata", {}),
    )
    db.session.add(event)
    db.session.commit()
    return jsonify({"event": event.to_dict()}), 201

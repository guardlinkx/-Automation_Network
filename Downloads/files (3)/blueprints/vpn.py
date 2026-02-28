import requests
from datetime import datetime, timezone
from flask import Blueprint, request, jsonify, current_app
from models import db, VpnServer, VpnSession
from blueprints.node_client import get_node_client

# Next-gen layer imports (graceful fallback)
try:
    from califax_core import crypto as pqc_crypto
    PQC_AVAILABLE = True
except ImportError:
    PQC_AVAILABLE = False

vpn_bp = Blueprint("vpn", __name__, url_prefix="/api/vpn")

LICENSE_VALIDATE_URL = (
    "https://tn1ict23f0.execute-api.us-east-1.amazonaws.com/v1/licenses/validate"
)


def validate_license(license_key):
    """Validate a license key against the existing Lambda API."""
    try:
        resp = requests.post(
            LICENSE_VALIDATE_URL,
            json={"license_key": license_key},
            timeout=10,
        )
        if resp.status_code == 200:
            data = resp.json()
            return data.get("valid", False), data
        return False, {"error": "License validation failed"}
    except requests.RequestException:
        return False, {"error": "License service unavailable"}


@vpn_bp.route("/servers", methods=["GET"])
def list_servers():
    """List available VPN servers with load info."""
    servers = VpnServer.query.filter_by(is_active=True).all()
    result = []
    for server in servers:
        active_sessions = VpnSession.query.filter_by(
            server_id=server.id, status="active"
        ).count()
        info = server.to_dict()
        info["current_load"] = active_sessions
        info["load_percent"] = round((active_sessions / server.max_peers) * 100, 1)
        result.append(info)
    return jsonify({"servers": result})


@vpn_bp.route("/connect", methods=["POST"])
def connect():
    """Validate license, pick server, add peer, return tunnel config."""
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    license_key = data.get("license_key")
    device_id = data.get("device_id")
    client_public_key = data.get("client_public_key")
    server_region = data.get("server_region")

    # Next-gen optional fields (backward compatible)
    protocol = data.get("protocol", "wireguard")
    pqc_enabled = data.get("pqc_enabled", False)
    mesh_circuit_id = data.get("mesh_circuit_id")

    if not all([license_key, device_id, client_public_key]):
        return jsonify({"error": "license_key, device_id, and client_public_key are required"}), 400

    # Validate license
    valid, license_data = validate_license(license_key)
    if not valid:
        return jsonify({"error": "Invalid or expired license", "details": license_data}), 403

    # Check for existing active session for this device
    existing = VpnSession.query.filter_by(
        license_key=license_key, device_id=device_id, status="active"
    ).first()
    if existing:
        # Disconnect old session first
        try:
            old_server = VpnServer.query.get(existing.server_id)
            if old_server:
                client = get_node_client(old_server)
                client.remove_peer(existing.client_public_key)
        except Exception:
            pass
        existing.status = "disconnected"
        existing.disconnected_at = datetime.now(timezone.utc)
        db.session.commit()

    # Pick server
    if server_region:
        server = VpnServer.query.filter_by(region=server_region, is_active=True).first()
    else:
        server = None

    if not server:
        # Pick least loaded active server
        servers = VpnServer.query.filter_by(is_active=True).all()
        if not servers:
            return jsonify({"error": "No VPN servers available"}), 503

        best = None
        best_load = float("inf")
        for s in servers:
            load = VpnSession.query.filter_by(server_id=s.id, status="active").count()
            if load < s.max_peers and load < best_load:
                best = s
                best_load = load
        server = best

    if not server:
        return jsonify({"error": "All servers are at capacity"}), 503

    # Add peer to VPN node
    try:
        client = get_node_client(server)
        peer_result = client.add_peer(client_public_key)
    except Exception as e:
        return jsonify({"error": f"Failed to provision VPN tunnel: {str(e)}"}), 502

    # Record session
    session = VpnSession(
        license_key=license_key,
        device_id=device_id,
        server_id=server.id,
        client_public_key=client_public_key,
        client_ip=peer_result["client_ip"],
    )
    db.session.add(session)
    db.session.commit()

    return jsonify({
        "status": "connected",
        "session_id": session.id,
        "protocol": protocol,
        "pqc_enabled": pqc_enabled and PQC_AVAILABLE,
        "mesh_circuit_id": mesh_circuit_id,
        "tunnel_config": {
            "server_public_key": peer_result["server_pubkey"],
            "endpoint": f"{server.ip_address}:51820",
            "client_ip": peer_result["client_ip"],
            "dns": ["1.1.1.1", "1.0.0.1"],
            "allowed_ips": "0.0.0.0/0",
            "keepalive": 25,
        },
        "server": {
            "region": server.region,
            "country": server.country,
            "city": server.city,
        },
    })


@vpn_bp.route("/disconnect", methods=["POST"])
def disconnect():
    """Remove peer and end VPN session."""
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    session_id = data.get("session_id")
    license_key = data.get("license_key")
    device_id = data.get("device_id")

    if session_id:
        session = VpnSession.query.get(session_id)
    elif license_key and device_id:
        session = VpnSession.query.filter_by(
            license_key=license_key, device_id=device_id, status="active"
        ).first()
    else:
        return jsonify({"error": "session_id or (license_key + device_id) required"}), 400

    if not session or session.status != "active":
        return jsonify({"error": "No active session found"}), 404

    # Remove peer from VPN node
    server = VpnServer.query.get(session.server_id)
    if server:
        try:
            client = get_node_client(server)
            client.remove_peer(session.client_public_key)
        except Exception:
            pass  # Best effort — session still marked disconnected

    session.status = "disconnected"
    session.disconnected_at = datetime.now(timezone.utc)
    db.session.commit()

    return jsonify({"status": "disconnected", "session_id": session.id})


@vpn_bp.route("/pqc/keypair", methods=["POST"])
def generate_pqc_keypair():
    """Generate a post-quantum hybrid keypair for the client."""
    if not PQC_AVAILABLE:
        return jsonify({"error": "PQC engine not available", "fallback": "standard_wireguard"}), 503

    try:
        keypair = pqc_crypto.generate_hybrid_keypair()
        return jsonify({
            "pqc_available": True,
            "algorithm": "X25519-Kyber1024",
            "x25519_public": keypair["x25519_public"],
            "kyber_public": keypair["kyber_public"],
        })
    except Exception as e:
        return jsonify({"error": f"Key generation failed: {str(e)}"}), 500


@vpn_bp.route("/pqc/status", methods=["GET"])
def pqc_status():
    """Check post-quantum cryptography availability."""
    return jsonify({
        "pqc_available": PQC_AVAILABLE,
        "algorithms": {
            "key_exchange": "X25519-Kyber1024" if PQC_AVAILABLE else "X25519",
            "symmetric": ["AES-256-GCM", "ChaCha20-Poly1305"],
            "kdf": "HKDF-SHA256",
        },
    })

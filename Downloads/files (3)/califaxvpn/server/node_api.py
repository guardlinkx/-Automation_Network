"""Califax VPN Node API — runs on each VPN server (port 8443)."""

import os
import time
import psutil
from functools import wraps
from flask import Flask, request, jsonify
from wg_manager import add_peer, remove_peer, get_status

app = Flask(__name__)

NODE_API_SECRET = os.getenv("NODE_API_SECRET", "change-me-in-production")
REGION = os.getenv("VPN_REGION", "us-east-1")
START_TIME = time.time()


def require_secret(f):
    """Authenticate requests via X-Node-Secret header."""
    @wraps(f)
    def wrapper(*args, **kwargs):
        secret = request.headers.get("X-Node-Secret", "")
        if secret != NODE_API_SECRET:
            return jsonify({"error": "Unauthorized"}), 401
        return f(*args, **kwargs)
    return wrapper


@app.route("/health", methods=["GET"])
@require_secret
def health():
    """Return node health metrics."""
    wg_status = get_status()
    return jsonify({
        "status": "healthy",
        "region": REGION,
        "uptime_seconds": int(time.time() - START_TIME),
        "connected_peers": wg_status["connected_peers"],
        "allocated_peers": wg_status["allocated_peers"],
        "cpu_percent": psutil.cpu_percent(interval=0.5),
        "memory_percent": psutil.virtual_memory().percent,
    })


@app.route("/peers", methods=["POST"])
@require_secret
def create_peer():
    """Add a new WireGuard peer."""
    data = request.get_json()
    if not data or "client_pubkey" not in data:
        return jsonify({"error": "client_pubkey required"}), 400

    client_pubkey = data["client_pubkey"]
    try:
        result = add_peer(client_pubkey)
        return jsonify(result), 201
    except RuntimeError as e:
        return jsonify({"error": str(e)}), 503
    except Exception as e:
        return jsonify({"error": f"Failed to add peer: {str(e)}"}), 500


@app.route("/peers", methods=["DELETE"])
@require_secret
def delete_peer():
    """Remove a WireGuard peer."""
    data = request.get_json()
    if not data or "client_pubkey" not in data:
        return jsonify({"error": "client_pubkey required"}), 400

    client_pubkey = data["client_pubkey"]
    try:
        result = remove_peer(client_pubkey)
        return jsonify(result)
    except Exception as e:
        return jsonify({"error": f"Failed to remove peer: {str(e)}"}), 500


if __name__ == "__main__":
    app.run(host="0.0.0.0", port=8443, ssl_context="adhoc")

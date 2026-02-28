#!/bin/bash
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

echo "[+] Califax VPN Node Bootstrap — ${vpn_region_label}"

# ── System updates ──
apt-get update -y
apt-get upgrade -y

# ── Install WireGuard ──
apt-get install -y wireguard wireguard-tools

# ── Generate server keypair ──
wg genkey | tee /etc/wireguard/server_private.key | wg pubkey > /etc/wireguard/server_public.key
chmod 600 /etc/wireguard/server_private.key

SERVER_PRIVKEY=$(cat /etc/wireguard/server_private.key)

# ── Create WireGuard config ──
cat > /etc/wireguard/wg0.conf <<EOF
[Interface]
Address = 10.100.0.1/24
ListenPort = 51820
PrivateKey = $SERVER_PRIVKEY
PostUp = iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE; iptables -A FORWARD -i wg0 -j ACCEPT; iptables -A FORWARD -o wg0 -j ACCEPT
PostDown = iptables -t nat -D POSTROUTING -o eth0 -j MASQUERADE; iptables -D FORWARD -i wg0 -j ACCEPT; iptables -D FORWARD -o wg0 -j ACCEPT
EOF

chmod 600 /etc/wireguard/wg0.conf

# Initialize peers file
echo '{}' > /etc/wireguard/peers.json

# ── Enable IP forwarding ──
echo "net.ipv4.ip_forward = 1" >> /etc/sysctl.conf
sysctl -p

# ── Start WireGuard ──
systemctl enable wg-quick@wg0
systemctl start wg-quick@wg0

# ── Install Python & Node API dependencies ──
apt-get install -y python3 python3-pip python3-venv

mkdir -p /opt/califaxvpn
cd /opt/califaxvpn

python3 -m venv venv
source venv/bin/activate

pip install flask psutil pyOpenSSL

# ── Write Node API code ──
cat > /opt/califaxvpn/wg_manager.py <<'PYEOF'
"""WireGuard CLI wrapper for managing peers on a VPN node."""

import subprocess
import ipaddress
import os
import json

WG_INTERFACE = "wg0"
WG_CONF_PATH = "/etc/wireguard/wg0.conf"
SUBNET = "10.100.0.0/24"
SERVER_IP = "10.100.0.1"
PEERS_FILE = "/etc/wireguard/peers.json"


def _run(cmd):
    result = subprocess.run(cmd, capture_output=True, text=True, check=True)
    return result.stdout.strip()


def _load_peers():
    if os.path.exists(PEERS_FILE):
        with open(PEERS_FILE, "r") as f:
            return json.load(f)
    return {}


def _save_peers(peers):
    with open(PEERS_FILE, "w") as f:
        json.dump(peers, f, indent=2)


def _next_available_ip():
    peers = _load_peers()
    used_ips = set(peers.values())
    used_ips.add(SERVER_IP)
    network = ipaddress.ip_network(SUBNET)
    for host in network.hosts():
        ip = str(host)
        if ip not in used_ips:
            return ip
    raise RuntimeError("No available IPs in subnet")


def get_server_pubkey():
    privkey = open("/etc/wireguard/server_private.key", "r").read().strip()
    result = subprocess.run(
        ["wg", "pubkey"], input=privkey,
        capture_output=True, text=True, check=True,
    )
    return result.stdout.strip()


def add_peer(client_pubkey):
    client_ip = _next_available_ip()
    _run(["wg", "set", WG_INTERFACE, "peer", client_pubkey, "allowed-ips", f"{client_ip}/32"])
    peers = _load_peers()
    peers[client_pubkey] = client_ip
    _save_peers(peers)
    _run(["wg-quick", "save", WG_INTERFACE])
    return {
        "client_ip": f"{client_ip}/32",
        "server_pubkey": get_server_pubkey(),
        "endpoint": f"{_get_public_ip()}:51820",
    }


def remove_peer(client_pubkey):
    _run(["wg", "set", WG_INTERFACE, "peer", client_pubkey, "remove"])
    peers = _load_peers()
    peers.pop(client_pubkey, None)
    _save_peers(peers)
    _run(["wg-quick", "save", WG_INTERFACE])
    return {"removed": client_pubkey}


def get_status():
    try:
        output = _run(["wg", "show", WG_INTERFACE, "dump"])
        lines = output.strip().split("\n")
        peer_count = max(0, len(lines) - 1)
    except subprocess.CalledProcessError:
        peer_count = 0
    return {
        "interface": WG_INTERFACE,
        "connected_peers": peer_count,
        "allocated_peers": len(_load_peers()),
    }


def _get_public_ip():
    cache = "/tmp/.califax_public_ip"
    if os.path.exists(cache):
        return open(cache).read().strip()
    try:
        token = subprocess.run(
            ["curl", "-s", "-X", "PUT",
             "http://169.254.169.254/latest/api/token",
             "-H", "X-aws-ec2-metadata-token-ttl-seconds: 300"],
            capture_output=True, text=True, timeout=3,
        ).stdout.strip()
        ip = subprocess.run(
            ["curl", "-s",
             "http://169.254.169.254/latest/meta-data/public-ipv4",
             "-H", f"X-aws-ec2-metadata-token: {token}"],
            capture_output=True, text=True, timeout=3,
        ).stdout.strip()
        with open(cache, "w") as f:
            f.write(ip)
        return ip
    except Exception:
        return "0.0.0.0"
PYEOF

cat > /opt/califaxvpn/node_api.py <<'PYEOF'
"""Califax VPN Node API."""

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
    data = request.get_json()
    if not data or "client_pubkey" not in data:
        return jsonify({"error": "client_pubkey required"}), 400
    try:
        result = add_peer(data["client_pubkey"])
        return jsonify(result), 201
    except RuntimeError as e:
        return jsonify({"error": str(e)}), 503
    except Exception as e:
        return jsonify({"error": f"Failed to add peer: {str(e)}"}), 500


@app.route("/peers", methods=["DELETE"])
@require_secret
def delete_peer():
    data = request.get_json()
    if not data or "client_pubkey" not in data:
        return jsonify({"error": "client_pubkey required"}), 400
    try:
        result = remove_peer(data["client_pubkey"])
        return jsonify(result)
    except Exception as e:
        return jsonify({"error": f"Failed to remove peer: {str(e)}"}), 500


if __name__ == "__main__":
    app.run(host="0.0.0.0", port=8443, ssl_context="adhoc")
PYEOF

# ── Create systemd service for Node API ──
cat > /etc/systemd/system/califaxvpn-node.service <<SVCEOF
[Unit]
Description=Califax VPN Node API
After=network.target wg-quick@wg0.service

[Service]
Type=simple
User=root
WorkingDirectory=/opt/califaxvpn
Environment="NODE_API_SECRET=${node_api_secret}"
Environment="VPN_REGION=${vpn_region_label}"
ExecStart=/opt/califaxvpn/venv/bin/python node_api.py
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
SVCEOF

systemctl daemon-reload
systemctl enable califaxvpn-node
systemctl start califaxvpn-node

echo "[+] Califax VPN Node bootstrap complete — ${vpn_region_label}"

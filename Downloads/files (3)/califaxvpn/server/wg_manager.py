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
    """Run a shell command and return stdout."""
    result = subprocess.run(cmd, capture_output=True, text=True, check=True)
    return result.stdout.strip()


def _load_peers():
    """Load allocated peers from file."""
    if os.path.exists(PEERS_FILE):
        with open(PEERS_FILE, "r") as f:
            return json.load(f)
    return {}


def _save_peers(peers):
    """Save allocated peers to file."""
    with open(PEERS_FILE, "w") as f:
        json.dump(peers, f, indent=2)


def _next_available_ip():
    """Find the next available IP in the subnet."""
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
    """Read the server's public key."""
    privkey_path = "/etc/wireguard/server_private.key"
    privkey = open(privkey_path, "r").read().strip()
    result = subprocess.run(
        ["wg", "pubkey"],
        input=privkey,
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout.strip()


def add_peer(client_pubkey):
    """Add a WireGuard peer. Returns assigned IP and server pubkey."""
    client_ip = _next_available_ip()

    _run([
        "wg", "set", WG_INTERFACE,
        "peer", client_pubkey,
        "allowed-ips", f"{client_ip}/32",
    ])

    # Persist peer mapping
    peers = _load_peers()
    peers[client_pubkey] = client_ip
    _save_peers(peers)

    # Sync to conf file for persistence across reboots
    _run(["wg-quick", "save", WG_INTERFACE])

    return {
        "client_ip": f"{client_ip}/32",
        "server_pubkey": get_server_pubkey(),
        "endpoint": f"{_get_public_ip()}:51820",
    }


def remove_peer(client_pubkey):
    """Remove a WireGuard peer."""
    _run(["wg", "set", WG_INTERFACE, "peer", client_pubkey, "remove"])

    peers = _load_peers()
    peers.pop(client_pubkey, None)
    _save_peers(peers)

    _run(["wg-quick", "save", WG_INTERFACE])
    return {"removed": client_pubkey}


def get_status():
    """Get WireGuard interface status."""
    try:
        output = _run(["wg", "show", WG_INTERFACE, "dump"])
        lines = output.strip().split("\n")
        # First line is interface info, rest are peers
        peer_count = max(0, len(lines) - 1)
    except subprocess.CalledProcessError:
        peer_count = 0

    return {
        "interface": WG_INTERFACE,
        "connected_peers": peer_count,
        "allocated_peers": len(_load_peers()),
    }


def _get_public_ip():
    """Get the node's public IP via metadata or cache."""
    cache = "/tmp/.califax_public_ip"
    if os.path.exists(cache):
        return open(cache).read().strip()
    try:
        # AWS EC2 instance metadata (IMDSv2)
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

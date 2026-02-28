#!/bin/bash
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

# System updates
apt-get update && apt-get upgrade -y
apt-get install -y wireguard curl jq build-essential pkg-config libssl-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# Configure WireGuard
wg genkey | tee /etc/wireguard/server_private.key | wg pubkey > /etc/wireguard/server_public.key
chmod 600 /etc/wireguard/server_private.key

PRIVATE_KEY=$(cat /etc/wireguard/server_private.key)

cat > /etc/wireguard/wg0.conf << WGEOF
[Interface]
PrivateKey = $PRIVATE_KEY
Address = 10.100.0.1/24
ListenPort = 51820
PostUp = iptables -A FORWARD -i %i -j ACCEPT; iptables -A FORWARD -o %i -j ACCEPT; iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
PostDown = iptables -D FORWARD -i %i -j ACCEPT; iptables -D FORWARD -o %i -j ACCEPT; iptables -t nat -D POSTROUTING -o eth0 -j MASQUERADE
WGEOF

# Enable IP forwarding
echo "net.ipv4.ip_forward = 1" >> /etc/sysctl.conf
sysctl -p

# Start WireGuard
systemctl enable wg-quick@wg0
systemctl start wg-quick@wg0

# Initialize peers file
echo '{}' > /etc/wireguard/peers.json

# Environment for califax-node
cat > /etc/califax-node.env << ENVEOF
NODE_API_SECRET=${node_secret}
VPN_REGION=${region}
LISTEN_ADDR=0.0.0.0:8443
MESH_PORT=${mesh_port}
RUST_LOG=info
ENVEOF

echo "[+] CalifaxVPN mesh relay node provisioned in ${region}"

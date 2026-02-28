import requests
from requests.exceptions import RequestException


class NodeClient:
    """HTTP client for communicating with VPN node APIs."""

    def __init__(self, ip_address, api_port, node_secret, timeout=10):
        self.base_url = f"https://{ip_address}:{api_port}"
        self.headers = {"X-Node-Secret": node_secret}
        self.timeout = timeout

    def add_peer(self, client_public_key):
        """Add a WireGuard peer to the node. Returns peer config."""
        resp = requests.post(
            f"{self.base_url}/peers",
            json={"client_pubkey": client_public_key},
            headers=self.headers,
            timeout=self.timeout,
            verify=False,
        )
        resp.raise_for_status()
        return resp.json()

    def remove_peer(self, client_public_key):
        """Remove a WireGuard peer from the node."""
        resp = requests.delete(
            f"{self.base_url}/peers",
            json={"client_pubkey": client_public_key},
            headers=self.headers,
            timeout=self.timeout,
            verify=False,
        )
        resp.raise_for_status()
        return resp.json()

    def health(self):
        """Get node health status."""
        resp = requests.get(
            f"{self.base_url}/health",
            headers=self.headers,
            timeout=self.timeout,
            verify=False,
        )
        resp.raise_for_status()
        return resp.json()


def get_node_client(server):
    """Create a NodeClient from a VpnServer model instance."""
    return NodeClient(
        ip_address=server.ip_address,
        api_port=server.api_port,
        node_secret=server.node_secret,
    )

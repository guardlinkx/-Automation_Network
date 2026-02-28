"""Decentralized Mesh Network — node management, circuit construction, relay control."""

from flask import Blueprint, request, jsonify
from models import db, MeshNode, MeshCircuit
from datetime import datetime, timezone
import secrets

mesh_bp = Blueprint("mesh", __name__, url_prefix="/api/mesh")

# Try importing Rust mesh engine
try:
    from califax_core import mesh as mesh_engine
    MESH_AVAILABLE = True
except ImportError:
    MESH_AVAILABLE = False


@mesh_bp.route("/status", methods=["GET"])
def mesh_status():
    """Get mesh network overview."""
    active_nodes = MeshNode.query.filter_by(is_active=True).count()
    total_circuits = MeshCircuit.query.filter_by(status="active").count()
    return jsonify({
        "mesh_available": MESH_AVAILABLE,
        "active_nodes": active_nodes,
        "active_circuits": total_circuits,
        "engine": "libp2p" if MESH_AVAILABLE else "database-only",
    })


@mesh_bp.route("/nodes", methods=["GET"])
def list_nodes():
    """List all active mesh relay nodes."""
    nodes = MeshNode.query.filter_by(is_active=True).all()
    return jsonify({"nodes": [n.to_dict() for n in nodes]})


@mesh_bp.route("/nodes/register", methods=["POST"])
def register_node():
    """Register a new mesh relay node."""
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    required = ["public_key", "endpoint", "region"]
    missing = [f for f in required if f not in data]
    if missing:
        return jsonify({"error": f"Missing fields: {missing}"}), 400

    node = MeshNode(
        peer_id=secrets.token_hex(16),
        public_key=data["public_key"],
        endpoint=data["endpoint"],
        region=data["region"],
        country=data.get("country", ""),
        bandwidth_mbps=data.get("bandwidth_mbps", 100),
        is_relay=data.get("is_relay", True),
        is_exit=data.get("is_exit", False),
    )
    db.session.add(node)
    db.session.commit()
    return jsonify({"node": node.to_dict()}), 201


@mesh_bp.route("/nodes/<int:node_id>/deregister", methods=["POST"])
def deregister_node(node_id):
    """Deregister a mesh relay node."""
    node = MeshNode.query.get_or_404(node_id)
    node.is_active = False
    node.last_seen = datetime.now(timezone.utc)
    db.session.commit()
    return jsonify({"status": "deregistered", "node_id": node_id})


@mesh_bp.route("/circuits/build", methods=["POST"])
def build_circuit():
    """Build a new multi-hop circuit through the mesh."""
    data = request.get_json() or {}
    hop_count = max(3, min(data.get("hop_count", 3), 7))
    preferred_regions = data.get("preferred_regions", [])
    exclude_nodes = data.get("exclude_nodes", [])

    # Select relay nodes for the circuit
    query = MeshNode.query.filter_by(is_active=True, is_relay=True)
    if exclude_nodes:
        query = query.filter(~MeshNode.id.in_(exclude_nodes))

    available_nodes = query.all()
    if len(available_nodes) < hop_count:
        return jsonify({"error": f"Not enough relay nodes. Need {hop_count}, have {len(available_nodes)}"}), 503

    # Build circuit: entry -> relays -> exit
    import random
    random.shuffle(available_nodes)

    # Prefer exit nodes for the last hop
    exit_nodes = [n for n in available_nodes if n.is_exit]
    relay_nodes = [n for n in available_nodes if not n.is_exit]

    circuit_nodes = []
    if exit_nodes:
        circuit_nodes = relay_nodes[:hop_count - 1] + [exit_nodes[0]]
    else:
        circuit_nodes = available_nodes[:hop_count]

    if len(circuit_nodes) < hop_count:
        circuit_nodes = available_nodes[:hop_count]

    circuit = MeshCircuit(
        circuit_id=secrets.token_hex(16),
        hop_count=len(circuit_nodes),
        node_ids=",".join(str(n.id) for n in circuit_nodes),
        entry_node_id=circuit_nodes[0].id,
        exit_node_id=circuit_nodes[-1].id,
    )
    db.session.add(circuit)
    db.session.commit()

    return jsonify({
        "circuit": circuit.to_dict(),
        "hops": [{"hop": i + 1, "node": n.to_dict()} for i, n in enumerate(circuit_nodes)],
    }), 201


@mesh_bp.route("/circuits", methods=["GET"])
def list_circuits():
    """List active circuits."""
    circuits = MeshCircuit.query.filter_by(status="active").all()
    return jsonify({"circuits": [c.to_dict() for c in circuits]})


@mesh_bp.route("/circuits/<circuit_id>/reshuffle", methods=["POST"])
def reshuffle_circuit(circuit_id):
    """Reshuffle an existing circuit with new relay nodes."""
    circuit = MeshCircuit.query.filter_by(circuit_id=circuit_id, status="active").first()
    if not circuit:
        return jsonify({"error": "Circuit not found"}), 404

    # Mark old circuit as reshuffled
    circuit.status = "reshuffled"
    circuit.reshuffled_at = datetime.now(timezone.utc)
    db.session.commit()

    # Build new circuit with same hop count, excluding current nodes
    old_node_ids = [int(x) for x in circuit.node_ids.split(",") if x]

    # Reuse build logic
    from flask import current_app
    with current_app.test_request_context(json={"hop_count": circuit.hop_count, "exclude_nodes": old_node_ids}):
        return build_circuit()


@mesh_bp.route("/circuits/<circuit_id>/destroy", methods=["POST"])
def destroy_circuit(circuit_id):
    """Tear down a circuit."""
    circuit = MeshCircuit.query.filter_by(circuit_id=circuit_id, status="active").first()
    if not circuit:
        return jsonify({"error": "Circuit not found"}), 404

    circuit.status = "destroyed"
    db.session.commit()
    return jsonify({"status": "destroyed", "circuit_id": circuit_id})

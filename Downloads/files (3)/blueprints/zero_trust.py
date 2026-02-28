"""Zero-Trust Enforcement — ZK proof verification, blockchain identity, warrant canary."""

from flask import Blueprint, request, jsonify
from models import db, BlockchainIdentity
from datetime import datetime, timezone

zt_bp = Blueprint("zero_trust", __name__, url_prefix="/api/zero-trust")

# Try importing Rust identity engine
try:
    from califax_core import identity as identity_engine
    ZT_AVAILABLE = True
except ImportError:
    ZT_AVAILABLE = False

# In-memory canary state (backed by blockchain in production)
_canary_state = {
    "alive": True,
    "last_chirp": datetime.now(timezone.utc).isoformat(),
    "chirp_interval_hours": 24,
    "operator": "CalifaxVPN",
    "message": "No warrants, subpoenas, or gag orders received.",
}


@zt_bp.route("/status", methods=["GET"])
def zt_status():
    """Return zero-trust engine status."""
    return jsonify({
        "zero_trust_available": ZT_AVAILABLE,
        "blockchain_network": "polygon" if ZT_AVAILABLE else "unavailable",
        "zk_proofs": ZT_AVAILABLE,
        "canary_alive": _canary_state["alive"],
    })


@zt_bp.route("/identity/register", methods=["POST"])
def register_identity():
    """Register a decentralized identity (DID)."""
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    wallet_address = data.get("wallet_address")
    public_key = data.get("public_key")

    if not wallet_address or not public_key:
        return jsonify({"error": "wallet_address and public_key required"}), 400

    # Check for existing identity
    existing = BlockchainIdentity.query.filter_by(wallet_address=wallet_address).first()
    if existing:
        return jsonify({"error": "Identity already registered"}), 409

    did = f"did:califax:{wallet_address[:10].lower()}"

    identity = BlockchainIdentity(
        wallet_address=wallet_address,
        public_key=public_key,
        did=did,
    )
    db.session.add(identity)
    db.session.commit()

    return jsonify({
        "did": did,
        "wallet_address": wallet_address,
        "status": "registered",
        "blockchain_tx": None if not ZT_AVAILABLE else "pending",
    }), 201


@zt_bp.route("/identity/resolve/<did>", methods=["GET"])
def resolve_identity(did):
    """Resolve a DID to its identity document."""
    identity = BlockchainIdentity.query.filter_by(did=did, is_active=True).first()
    if not identity:
        return jsonify({"error": "DID not found"}), 404
    return jsonify(identity.to_dict())


@zt_bp.route("/identity/verify", methods=["POST"])
def verify_identity():
    """Verify an identity using ZK proof (alternative to license key)."""
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    did = data.get("did")
    zk_proof = data.get("zk_proof")  # hex-encoded proof

    if not did or not zk_proof:
        return jsonify({"error": "did and zk_proof required"}), 400

    identity = BlockchainIdentity.query.filter_by(did=did, is_active=True).first()
    if not identity:
        return jsonify({"error": "DID not found"}), 404

    if ZT_AVAILABLE:
        try:
            verified = identity_engine.verify_zk_proof(zk_proof, identity.public_key)
            return jsonify({"verified": verified, "did": did, "method": "groth16"})
        except Exception as e:
            return jsonify({"error": f"ZK verification failed: {str(e)}"}), 500

    # Stub verification: accept if proof is non-empty hex
    try:
        bytes.fromhex(zk_proof)
        verified = len(zk_proof) >= 64
    except ValueError:
        verified = False

    return jsonify({
        "verified": verified,
        "did": did,
        "method": "stub",
        "warning": "ZK engine not available — using stub verification",
    })


@zt_bp.route("/canary", methods=["GET"])
def canary_status():
    """Check warrant canary status."""
    return jsonify({
        "canary": _canary_state,
        "blockchain_verified": ZT_AVAILABLE,
    })


@zt_bp.route("/canary/chirp", methods=["POST"])
def canary_chirp():
    """Update the warrant canary (admin only, must be called every 24h)."""
    # In production, this would require admin auth and blockchain tx
    data = request.get_json() or {}
    _canary_state["last_chirp"] = datetime.now(timezone.utc).isoformat()
    _canary_state["message"] = data.get("message", _canary_state["message"])
    return jsonify({"status": "chirped", "canary": _canary_state})


@zt_bp.route("/access/check", methods=["POST"])
def check_access():
    """Zero-trust access decision for a resource."""
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    did = data.get("did")
    resource = data.get("resource")
    context = data.get("context", {})

    if not did or not resource:
        return jsonify({"error": "did and resource required"}), 400

    identity = BlockchainIdentity.query.filter_by(did=did, is_active=True).first()
    if not identity:
        return jsonify({"allowed": False, "reason": "identity_not_found"})

    # Zero-trust: verify every time, trust nothing
    trust_score = identity.trust_score

    # Adjust based on context
    if context.get("new_device"):
        trust_score -= 20
    if context.get("unusual_location"):
        trust_score -= 15
    if context.get("off_hours"):
        trust_score -= 10

    allowed = trust_score >= 50

    return jsonify({
        "allowed": allowed,
        "trust_score": trust_score,
        "did": did,
        "resource": resource,
        "factors_evaluated": ["identity", "device", "location", "time"],
    })

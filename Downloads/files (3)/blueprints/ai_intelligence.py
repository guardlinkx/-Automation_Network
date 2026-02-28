"""AI Intelligence Core — threat analysis, anomaly detection, and routing optimization."""

from flask import Blueprint, request, jsonify
from datetime import datetime, timezone

ai_bp = Blueprint("ai_intelligence", __name__, url_prefix="/api/ai")

# Try to import Rust AI engine via PyO3, fallback to stub
try:
    from califax_core import ai as ai_engine
    AI_AVAILABLE = True
except ImportError:
    AI_AVAILABLE = False


@ai_bp.route("/status", methods=["GET"])
def ai_status():
    """Return AI engine availability and loaded models."""
    return jsonify({
        "ai_available": AI_AVAILABLE,
        "models": {
            "threat_detector": AI_AVAILABLE,
            "anomaly_scorer": AI_AVAILABLE,
            "route_optimizer": AI_AVAILABLE,
        },
        "engine": "onnxruntime" if AI_AVAILABLE else "unavailable",
    })


@ai_bp.route("/threat/analyze", methods=["POST"])
def analyze_threat():
    """Analyze network flow for threats."""
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    required = ["packet_size", "flow_duration", "packet_rate", "byte_rate"]
    missing = [f for f in required if f not in data]
    if missing:
        return jsonify({"error": f"Missing fields: {missing}"}), 400

    if AI_AVAILABLE:
        try:
            result = ai_engine.analyze_threat(data)
            return jsonify(result)
        except Exception as e:
            return jsonify({"error": f"AI inference failed: {str(e)}"}), 500

    # Heuristic fallback when AI not available
    threat_score = 0.0
    reasons = []
    if data.get("packet_rate", 0) > 10000:
        threat_score += 0.4
        reasons.append("high_packet_rate")
    if data.get("payload_entropy", 0) > 7.5:
        threat_score += 0.3
        reasons.append("high_payload_entropy")
    if data.get("port_entropy", 0) > 4.0:
        threat_score += 0.3
        reasons.append("high_port_entropy")

    return jsonify({
        "is_threat": threat_score > 0.5,
        "confidence": min(threat_score, 1.0),
        "threat_type": reasons[0] if reasons else None,
        "analysis_mode": "heuristic",
        "details": reasons,
    })


@ai_bp.route("/anomaly/score", methods=["POST"])
def score_anomaly():
    """Score network behavior for anomalies."""
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    if AI_AVAILABLE:
        try:
            result = ai_engine.score_anomaly(data)
            return jsonify(result)
        except Exception as e:
            return jsonify({"error": f"AI inference failed: {str(e)}"}), 500

    # Heuristic fallback
    score = 0.0
    if data.get("bytes_per_second", 0) > 100_000_000:
        score += 0.3
    if data.get("failed_connections", 0) > 50:
        score += 0.3
    if data.get("dns_query_rate", 0) > 100:
        score += 0.2
    if data.get("reconnect_frequency", 0) > 10:
        score += 0.2

    return jsonify({
        "score": min(score, 1.0),
        "is_anomalous": score > 0.5,
        "analysis_mode": "heuristic",
    })


@ai_bp.route("/routing/recommend", methods=["POST"])
def recommend_route():
    """AI-optimized server/route recommendation."""
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    candidates = data.get("candidates", [])
    if not candidates:
        return jsonify({"error": "candidates list required"}), 400

    if AI_AVAILABLE:
        try:
            result = ai_engine.recommend_route(candidates)
            return jsonify(result)
        except Exception as e:
            return jsonify({"error": f"AI inference failed: {str(e)}"}), 500

    # Heuristic: pick lowest load
    best_idx = 0
    best_score = float("inf")
    for i, c in enumerate(candidates):
        score = c.get("current_load", 50) + c.get("hop_count", 1) * 10
        if score < best_score:
            best_score = score
            best_idx = i

    return jsonify({
        "recommended_index": best_idx,
        "predicted_latency_ms": best_score,
        "analysis_mode": "heuristic",
    })


@ai_bp.route("/protocol/recommend", methods=["POST"])
def recommend_protocol():
    """AI-driven protocol selection based on network conditions."""
    data = request.get_json()
    if not data:
        return jsonify({"error": "Request body required"}), 400

    latency = data.get("latency_ms", 50)
    packet_loss = data.get("packet_loss_percent", 0)
    is_censored = data.get("is_censored_network", False)
    is_public_wifi = data.get("is_public_wifi", False)
    dpi_detected = data.get("dpi_detected", False)

    if dpi_detected or is_censored:
        protocol = "chameleon"
        reason = "DPI/censorship detected — using Chameleon obfuscation"
    elif is_public_wifi:
        protocol = "obfuscated_wireguard"
        reason = "Public Wi-Fi detected — using obfuscated WireGuard"
    elif packet_loss > 5:
        protocol = "ikev2"
        reason = "High packet loss — IKEv2 handles mobility better"
    elif latency < 20:
        protocol = "wireguard"
        reason = "Low latency network — WireGuard optimal"
    else:
        protocol = "wireguard"
        reason = "Default — WireGuard provides best performance"

    return jsonify({
        "recommended_protocol": protocol,
        "reason": reason,
        "fallback_chain": ["wireguard", "obfuscated_wireguard", "shadowsocks", "ikev2", "chameleon"],
    })

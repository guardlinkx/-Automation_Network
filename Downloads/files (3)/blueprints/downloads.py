import os
from datetime import datetime, timezone

from flask import Blueprint, abort, current_app, jsonify, request, send_file
from models import TrialRegistration, db

downloads_bp = Blueprint("downloads", __name__, url_prefix="/downloads")

DOWNLOAD_FILES = {
    "guarddrive": "DriveGuardPro-v2.0.0-trial.zip",
}


@downloads_bp.route("/register/<product_slug>", methods=["POST"])
def register(product_slug):
    if product_slug not in DOWNLOAD_FILES:
        return jsonify({"error": "No downloadable file for this product"}), 404

    data = request.get_json(silent=True) or {}
    name = (data.get("name") or "").strip()
    email = (data.get("email") or "").strip()

    if not name or not email:
        return jsonify({"error": "Name and email are required"}), 400

    reg = TrialRegistration(name=name, email=email, product_slug=product_slug)
    db.session.add(reg)
    db.session.commit()

    return jsonify({"token": reg.download_token}), 201


@downloads_bp.route("/get/<token>")
def download(token):
    reg = TrialRegistration.query.filter_by(download_token=token).first()
    if not reg:
        abort(404, description="Invalid download link.")

    now = datetime.now(timezone.utc)
    expires = reg.token_expires_at
    if expires.tzinfo is None:
        expires = expires.replace(tzinfo=timezone.utc)
    if now > expires:
        abort(410, description="This download link has expired. Please register again.")

    filename = DOWNLOAD_FILES.get(reg.product_slug)
    if not filename:
        abort(404, description="Download file not found.")

    downloads_dir = os.path.join(current_app.root_path, "downloads")
    filepath = os.path.join(downloads_dir, filename)
    if not os.path.isfile(filepath):
        abort(404, description="Download file not found on server.")

    reg.downloaded = True
    db.session.commit()

    return send_file(filepath, as_attachment=True, download_name=filename)

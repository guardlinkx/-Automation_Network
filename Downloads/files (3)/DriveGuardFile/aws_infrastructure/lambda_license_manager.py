"""
DriveGuard Pro - License Manager (AWS Lambda)

This Lambda sits behind API Gateway at /licenses/{action} and handles:
  - POST /licenses/validate  — check license validity
  - POST /licenses/activate  — bind a machine to a license
  - POST /licenses/deactivate — remove a machine binding

License data is stored in Firebase Firestore (collection: "licenses").
Firebase credentials are loaded from AWS Secrets Manager at cold start.

Environment variables (set by Terraform):
    FIREBASE_SECRET_NAME  - Secrets Manager secret holding the service account JSON
    FIREBASE_PROJECT_ID   - Firebase/GCP project ID
"""

import json
import logging
import os
import uuid
from datetime import datetime, timezone

import boto3

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# ── Firebase init (cold start, cached) ─────────────────────────────

_firestore_db = None


def _get_firestore():
    global _firestore_db
    if _firestore_db is not None:
        return _firestore_db

    import base64
    import firebase_admin
    from firebase_admin import credentials, firestore

    cred_b64 = os.environ["FIREBASE_CREDENTIALS_B64"]
    cred_dict = json.loads(base64.b64decode(cred_b64))

    cred = credentials.Certificate(cred_dict)
    firebase_admin.initialize_app(cred, {
        "projectId": os.environ.get("FIREBASE_PROJECT_ID", ""),
    })
    _firestore_db = firestore.client()
    return _firestore_db


# ── Handler ────────────────────────────────────────────────────────

def lambda_handler(event, context):
    """API Gateway proxy integration entry point."""
    path_params = event.get("pathParameters") or {}
    action = path_params.get("action", "")

    try:
        body = json.loads(event.get("body", "{}"))
    except (json.JSONDecodeError, TypeError):
        return _response(400, {"error": "Invalid JSON body."})

    license_key = body.get("license_key", "").strip()
    if not license_key:
        return _response(400, {"error": "license_key is required."})

    if action == "validate":
        return _handle_validate(license_key, body)
    elif action == "activate":
        return _handle_activate(license_key, body)
    elif action == "deactivate":
        return _handle_deactivate(license_key, body)
    else:
        return _response(404, {"error": f"Unknown action: {action}"})


# ── Validate ───────────────────────────────────────────────────────

def _handle_validate(license_key, body):
    """Validate a license key and optionally verify machine activation."""
    db = _get_firestore()
    doc = db.collection("licenses").document(license_key).get()

    if not doc.exists:
        return _response(200, {"valid": False, "error": "Invalid license key."})

    data = doc.to_dict()

    if data.get("status") != "active":
        return _response(200, {
            "valid": False,
            "error": f"License is {data.get('status', 'invalid')}.",
        })

    # Check expiry
    expires_at = data.get("expires_at")
    if expires_at:
        exp_dt = expires_at if hasattr(expires_at, "isoformat") else None
        if exp_dt and datetime.now(timezone.utc) > exp_dt.replace(tzinfo=timezone.utc):
            return _response(200, {"valid": False, "error": "License has expired."})

    # If instance_id provided, verify this machine is activated
    instance_id = body.get("instance_id", "").strip()
    if instance_id:
        activations = data.get("activations", [])
        found = any(a.get("instance_id") == instance_id for a in activations)
        if not found:
            return _response(200, {
                "valid": False,
                "error": "This machine is not activated for this license.",
            })

    return _response(200, {
        "valid": True,
        "license_type": data.get("license_type", "standalone"),
        "expires_at": expires_at.isoformat() if hasattr(expires_at, "isoformat") else (expires_at or ""),
        "customer_name": data.get("customer_name", ""),
        "features": data.get("features", []),
    })


# ── Activate ──────────────────────────────────────────────────────

def _handle_activate(license_key, body):
    """Activate a license on a specific machine."""
    machine_id = body.get("instance_name", "").strip() or body.get("machine_id", "").strip()
    if not machine_id:
        return _response(400, {"error": "instance_name (machine_id) is required."})

    db = _get_firestore()
    doc_ref = db.collection("licenses").document(license_key)

    @_firestore_transaction
    def do_activate(transaction):
        doc = doc_ref.get(transaction=transaction)
        if not doc.exists:
            return _response(200, {"activated": False, "error": "Invalid license key."})

        data = doc.to_dict()

        if data.get("status") != "active":
            return _response(200, {
                "activated": False,
                "error": f"License is {data.get('status', 'invalid')}.",
            })

        # Check expiry
        expires_at = data.get("expires_at")
        if expires_at:
            exp_dt = expires_at if hasattr(expires_at, "isoformat") else None
            if exp_dt and datetime.now(timezone.utc) > exp_dt.replace(tzinfo=timezone.utc):
                return _response(200, {"activated": False, "error": "License has expired."})

        activations = data.get("activations", [])
        max_activations = data.get("max_activations", 3)

        # Check if this machine is already activated
        for a in activations:
            if a.get("machine_id") == machine_id:
                # Already activated on this machine — return existing instance
                return _response(200, {
                    "activated": True,
                    "license_type": data.get("license_type", "standalone"),
                    "expires_at": expires_at.isoformat() if hasattr(expires_at, "isoformat") else (expires_at or ""),
                    "customer_name": data.get("customer_name", ""),
                    "features": data.get("features", []),
                    "instance": {"id": a["instance_id"]},
                })

        # Check activation limit
        if len(activations) >= max_activations:
            return _response(200, {
                "activated": False,
                "error": "Activation limit reached. Deactivate another machine first.",
            })

        # Add new activation
        new_instance_id = str(uuid.uuid4())
        activations.append({
            "machine_id": machine_id,
            "instance_id": new_instance_id,
            "activated_at": datetime.now(timezone.utc).isoformat(),
        })

        transaction.update(doc_ref, {"activations": activations})

        return _response(200, {
            "activated": True,
            "license_type": data.get("license_type", "standalone"),
            "expires_at": expires_at.isoformat() if hasattr(expires_at, "isoformat") else (expires_at or ""),
            "customer_name": data.get("customer_name", ""),
            "features": data.get("features", []),
            "instance": {"id": new_instance_id},
        })

    try:
        return do_activate()
    except Exception as e:
        logger.error(f"Activation failed: {e}")
        return _response(500, {"activated": False, "error": "Activation failed."})


# ── Deactivate ────────────────────────────────────────────────────

def _handle_deactivate(license_key, body):
    """Remove a machine activation from a license."""
    instance_id = body.get("instance_id", "").strip()
    if not instance_id:
        return _response(400, {"error": "instance_id is required."})

    db = _get_firestore()
    doc_ref = db.collection("licenses").document(license_key)

    @_firestore_transaction
    def do_deactivate(transaction):
        doc = doc_ref.get(transaction=transaction)
        if not doc.exists:
            return _response(200, {"deactivated": False, "error": "Invalid license key."})

        data = doc.to_dict()
        activations = data.get("activations", [])

        new_activations = [a for a in activations if a.get("instance_id") != instance_id]

        if len(new_activations) == len(activations):
            return _response(200, {
                "deactivated": False,
                "error": "Instance not found in activations.",
            })

        transaction.update(doc_ref, {"activations": new_activations})

        return _response(200, {"deactivated": True})

    try:
        return do_deactivate()
    except Exception as e:
        logger.error(f"Deactivation failed: {e}")
        return _response(500, {"deactivated": False, "error": "Deactivation failed."})


# ── Helpers ────────────────────────────────────────────────────────

def _firestore_transaction(func):
    """Decorator that runs a function inside a Firestore transaction."""
    def wrapper():
        db = _get_firestore()
        transaction = db.transaction()

        @_get_firestore_transactional()
        def run_in_transaction(transaction):
            return func(transaction)

        return run_in_transaction(transaction)
    return wrapper


def _get_firestore_transactional():
    """Get the firestore transactional decorator."""
    from google.cloud.firestore_v1 import transactional
    return transactional


def _response(status_code, body):
    """Format an API Gateway proxy response."""
    return {
        "statusCode": status_code,
        "headers": {
            "Content-Type": "application/json",
            "Access-Control-Allow-Origin": "*",
        },
        "body": json.dumps(body, default=str),
    }

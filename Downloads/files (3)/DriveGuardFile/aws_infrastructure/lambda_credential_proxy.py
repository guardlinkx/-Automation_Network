"""
DriveGuard Pro - S3 Credential Proxy (AWS Lambda)

This Lambda function sits behind API Gateway and:
1. Receives a license_key from the DriveGuard Pro desktop app
2. Validates it against Firebase Firestore (licenses collection)
3. If the key is a valid subscription, issues temporary AWS STS credentials
   scoped to that subscriber's prefix in the shared S3 bucket

Deploy via Terraform (see terraform/ directory).

Environment variables (set by Terraform):
    S3_BUCKET_NAME       - The shared backup bucket (e.g. driveguard-backups)
    S3_UPLOAD_ROLE_ARN   - IAM role ARN that grants scoped S3 access
    CREDENTIAL_DURATION  - STS credential lifetime in seconds (default 3600)
    FIREBASE_SECRET_NAME - Secrets Manager secret holding Firebase service account JSON
    FIREBASE_PROJECT_ID  - Firebase/GCP project ID
"""

import hashlib
import json
import logging
import os
from datetime import datetime, timezone

import boto3

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# ── Config from environment ──────────────────────────────────────
S3_BUCKET = os.environ["S3_BUCKET_NAME"]
UPLOAD_ROLE_ARN = os.environ["S3_UPLOAD_ROLE_ARN"]
CREDENTIAL_DURATION = int(os.environ.get("CREDENTIAL_DURATION", "3600"))

sts_client = boto3.client("sts")

# ── Firebase init (cold start, cached) ───────────────────────────

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


def lambda_handler(event, context):
    """API Gateway proxy integration entry point."""
    try:
        body = json.loads(event.get("body", "{}"))
    except (json.JSONDecodeError, TypeError):
        return _response(400, {"error": "Invalid JSON body."})

    license_key = body.get("license_key", "").strip()
    instance_id = body.get("instance_id", "").strip()

    if not license_key:
        return _response(400, {"error": "license_key is required."})

    # ── Step 1: Validate against Firestore ───────────────────────
    valid, license_data, error = _validate_license(license_key, instance_id)
    if not valid:
        return _response(403, {"error": error})

    # Check this is a subscription license (cloud backup requires subscription)
    if license_data.get("license_type") != "subscription":
        return _response(403, {
            "error": "Cloud backup is only available for Subscription licenses."
        })

    # ── Step 2: Derive subscriber prefix ─────────────────────────
    subscriber_prefix = _subscriber_prefix(license_key)

    # ── Step 3: Issue scoped STS credentials ─────────────────────
    try:
        scoped_policy = json.dumps({
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Action": [
                        "s3:PutObject",
                        "s3:GetObject",
                        "s3:DeleteObject",
                        "s3:ListBucket",
                        "s3:AbortMultipartUpload",
                        "s3:ListMultipartUploadParts",
                    ],
                    "Resource": [
                        f"arn:aws:s3:::{S3_BUCKET}/{subscriber_prefix}/*",
                    ],
                },
                {
                    "Effect": "Allow",
                    "Action": ["s3:ListBucket"],
                    "Resource": f"arn:aws:s3:::{S3_BUCKET}",
                    "Condition": {
                        "StringLike": {
                            "s3:prefix": [f"{subscriber_prefix}/*"]
                        }
                    },
                },
            ],
        })

        assumed = sts_client.assume_role(
            RoleArn=UPLOAD_ROLE_ARN,
            RoleSessionName=f"dg-{subscriber_prefix[:12]}",
            Policy=scoped_policy,
            DurationSeconds=CREDENTIAL_DURATION,
        )

        creds = assumed["Credentials"]

        return _response(200, {
            "bucket": S3_BUCKET,
            "prefix": f"{subscriber_prefix}/",
            "region": os.environ.get("AWS_REGION", "us-east-1"),
            "credentials": {
                "access_key_id": creds["AccessKeyId"],
                "secret_access_key": creds["SecretAccessKey"],
                "session_token": creds["SessionToken"],
                "expiration": creds["Expiration"].isoformat(),
            },
        })

    except Exception as e:
        logger.error(f"STS assume_role failed: {e}")
        return _response(500, {"error": "Failed to generate credentials."})


# ── Helpers ──────────────────────────────────────────────────────

def _validate_license(license_key, instance_id=""):
    """
    Validate a license key against Firebase Firestore.

    Returns:
        (is_valid, license_data_dict, error_message)
    """
    try:
        db = _get_firestore()
        doc = db.collection("licenses").document(license_key).get()

        if not doc.exists:
            return False, {}, "Invalid license key."

        data = doc.to_dict()

        if data.get("status") != "active":
            return False, {}, f"License is {data.get('status', 'invalid')}."

        # Check expiry
        expires_at = data.get("expires_at")
        if expires_at:
            exp_dt = expires_at if hasattr(expires_at, "isoformat") else None
            if exp_dt and datetime.now(timezone.utc) > exp_dt.replace(tzinfo=timezone.utc):
                return False, {}, "License has expired."

        # If instance_id provided, verify this machine is activated
        if instance_id:
            activations = data.get("activations", [])
            found = any(a.get("instance_id") == instance_id for a in activations)
            if not found:
                return False, {}, "This machine is not activated for this license."

        return True, {
            "license_type": data.get("license_type", "standalone"),
            "expires_at": expires_at.isoformat() if hasattr(expires_at, "isoformat") else (expires_at or ""),
            "customer_name": data.get("customer_name", ""),
        }, ""

    except Exception as e:
        logger.error(f"Firestore validation failed: {e}")
        return False, {}, f"License validation failed: {e}"


def _subscriber_prefix(license_key):
    """Derive a unique S3 prefix from the license key."""
    return hashlib.sha256(license_key.encode()).hexdigest()[:16]


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

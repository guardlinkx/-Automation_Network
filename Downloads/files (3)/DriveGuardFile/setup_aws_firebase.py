"""
DriveGuard Pro - Firebase Firestore Setup Script

This script:
1. Reads your Firebase service account JSON
2. Seeds a test license in Firestore

The Firebase credentials are passed to Lambda via Terraform as a
base64-encoded environment variable (no Secrets Manager needed).

Usage:
    python setup_aws_firebase.py
"""

import json
import os
import sys


def main():
    print("=" * 56)
    print(" DriveGuard Pro - Firebase Setup")
    print("=" * 56)
    print()

    # ── Step 1: Firebase Service Account JSON ────────────────
    print("[1/2] Firebase Service Account")
    print("-" * 40)
    default_path = os.path.join(
        os.path.expanduser("~"), "Downloads", "firebase-service-account.json"
    )
    firebase_path = input(f"  Path to service account JSON [{default_path}]: ").strip()
    if not firebase_path:
        firebase_path = default_path

    if not os.path.isfile(firebase_path):
        # Try to find it in Downloads
        downloads_dir = os.path.join(os.path.expanduser("~"), "Downloads")
        if os.path.isdir(downloads_dir):
            for f in os.listdir(downloads_dir):
                if f.endswith(".json") and "firebase" in f.lower():
                    candidate = os.path.join(downloads_dir, f)
                    confirm = input(f"  Found: {candidate} — use this? [Y/n]: ").strip()
                    if confirm.lower() != "n":
                        firebase_path = candidate
                        break

    if not os.path.isfile(firebase_path):
        print(f"ERROR: File not found: {firebase_path}")
        print()
        print("Please download your Firebase service account key first:")
        print("  1. Go to Firebase Console -> Project Settings -> Service accounts")
        print("  2. Click 'Generate new private key'")
        print("  3. Save the file and re-run this script")
        sys.exit(1)

    with open(firebase_path, "r", encoding="utf-8") as f:
        firebase_json = f.read()

    try:
        sa_data = json.loads(firebase_json)
        project_id = sa_data.get("project_id", "")
        if "private_key" not in sa_data:
            print("ERROR: This doesn't look like a Firebase service account JSON.")
            sys.exit(1)
        print(f"  Project ID: {project_id}")
        print(f"  Client email: {sa_data.get('client_email', 'N/A')}")
    except json.JSONDecodeError:
        print("ERROR: Invalid JSON file.")
        sys.exit(1)
    print()

    # ── Step 2: Seed a test license in Firestore ─────────────
    print("[2/2] Seed Test License in Firestore")
    print("-" * 40)
    _seed_test_license(firebase_path, project_id)

    print()
    print("=" * 56)
    print(" Setup complete!")
    print("=" * 56)
    print()
    print("Next steps:")
    print(f"  1. Run: bash build_lambda_packages.sh")
    print(f"  2. cd terraform && terraform init")
    print(f"  3. terraform apply \\")
    print(f"       -var='firebase_project_id={project_id}' \\")
    print(f"       -var='firebase_credentials_file={firebase_path}' \\")
    print(f"       -var='bucket_name=YOUR-UNIQUE-BUCKET-NAME'")
    print(f"  4. Copy the output URLs into cloud_backend.py and license_manager.py")
    print()


def _seed_test_license(firebase_path, project_id):
    """Create a test license document in Firestore."""
    try:
        import firebase_admin
        from firebase_admin import credentials, firestore
    except ImportError:
        print("  Installing firebase-admin...")
        import subprocess
        subprocess.check_call(
            [sys.executable, "-m", "pip", "install", "firebase-admin", "-q"]
        )
        import firebase_admin
        from firebase_admin import credentials, firestore

    cred = credentials.Certificate(firebase_path)
    try:
        firebase_admin.get_app()
    except ValueError:
        firebase_admin.initialize_app(cred, {"projectId": project_id})

    db = firestore.client()

    license_key = input("  License key (e.g. DGPRO-TEST-1234-5678): ").strip()
    if not license_key:
        license_key = "DGPRO-TEST-1234-5678"

    license_type = input("  License type [subscription]: ").strip() or "subscription"
    customer_name = input("  Customer name [Test User]: ").strip() or "Test User"
    customer_email = input("  Customer email [test@example.com]: ").strip() or "test@example.com"

    from datetime import datetime, timedelta, timezone
    expires_at = datetime.now(timezone.utc) + timedelta(days=365)

    doc_data = {
        "license_type": license_type,
        "status": "active",
        "customer_name": customer_name,
        "customer_email": customer_email,
        "features": ["cloud_backup", "scheduled_backup"],
        "expires_at": expires_at,
        "created_at": datetime.now(timezone.utc),
        "max_activations": 3,
        "activations": [],
    }

    db.collection("licenses").document(license_key).set(doc_data)
    print()
    print(f"  Created license: {license_key}")
    print(f"    Type: {license_type}")
    print(f"    Status: active")
    print(f"    Expires: {expires_at.strftime('%Y-%m-%d')}")
    print(f"    Max activations: 3")


if __name__ == "__main__":
    main()

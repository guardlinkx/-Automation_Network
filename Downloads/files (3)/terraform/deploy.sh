#!/bin/bash
# GuardLinkX — Deploy Flask app to EC2
# Usage: ./deploy.sh <server-ip> <ssh-key-path>
#
# Run from the project root directory:
#   ./terraform/deploy.sh 1.2.3.4 ~/.ssh/guardlinkx-key

set -euo pipefail

if [ $# -lt 2 ]; then
    echo "Usage: $0 <server-ip> <ssh-key-path>"
    echo "Example: $0 54.123.45.67 ~/.ssh/guardlinkx-key"
    exit 1
fi

SERVER=$1
KEY=$2
REMOTE_USER="ubuntu"
APP_DIR="/opt/guardlinkx/app"

echo "=== Deploying GuardLinkX to $SERVER ==="

# Upload app code via rsync (exclude dev/local files)
echo "[1/4] Uploading application code..."
rsync -avz --delete \
    -e "ssh -i $KEY -o StrictHostKeyChecking=no" \
    --exclude='__pycache__' \
    --exclude='*.pyc' \
    --exclude='instance/' \
    --exclude='.env' \
    --exclude='.env.example' \
    --exclude='DriveGuardFile/' \
    --exclude='terraform/' \
    --exclude='downloads/*.zip' \
    --exclude='.git/' \
    --exclude='.claude/' \
    --exclude='*.db' \
    ./ "$REMOTE_USER@$SERVER:/tmp/guardlinkx-deploy/"

echo "[2/4] Installing on server..."
ssh -i "$KEY" -o StrictHostKeyChecking=no "$REMOTE_USER@$SERVER" << 'REMOTE_SCRIPT'
    set -euo pipefail

    # Copy code to app directory
    sudo mkdir -p /opt/guardlinkx/app
    sudo cp -r /tmp/guardlinkx-deploy/* /opt/guardlinkx/app/
    sudo chown -R guardlinkx:guardlinkx /opt/guardlinkx/

    # Create downloads directory
    sudo -u guardlinkx mkdir -p /opt/guardlinkx/app/downloads

    # Install Python dependencies
    echo "[3/4] Installing Python dependencies..."
    sudo -u guardlinkx /opt/guardlinkx/venv/bin/pip install --quiet -r /opt/guardlinkx/app/requirements.txt

    # Initialize database tables
    echo "[4/4] Initializing database..."
    sudo -u guardlinkx /opt/guardlinkx/venv/bin/python -c "
import sys
sys.path.insert(0, '/opt/guardlinkx/app')
from app import create_app
app = create_app()
with app.app_context():
    from models import db
    db.create_all()
    print('Database tables created.')
"

    # Restart the service
    sudo systemctl restart guardlinkx
    echo ""
    echo "=== Checking service status ==="
    sleep 2
    sudo systemctl status guardlinkx --no-pager || true

    # Clean up
    rm -rf /tmp/guardlinkx-deploy
REMOTE_SCRIPT

echo ""
echo "=== Deployment complete! ==="
echo "Site: http://$SERVER (or https://${3:-califaxvpn.guardlinkx.com} after Certbot)"

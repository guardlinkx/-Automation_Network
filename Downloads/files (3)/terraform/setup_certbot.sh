#!/bin/bash
# GuardLinkX — Setup Let's Encrypt SSL via Certbot
# Usage: ./setup_certbot.sh <server-ip> <ssh-key-path> [domain] [email]
#
# Prerequisites:
#   1. DNS A record for the domain must point to the server IP
#   2. DNS must have propagated (check with: dig califaxvpn.guardlinkx.com)
#   3. HTTP (port 80) must be accessible from the internet

set -euo pipefail

if [ $# -lt 2 ]; then
    echo "Usage: $0 <server-ip> <ssh-key-path> [domain] [email]"
    echo "Example: $0 54.123.45.67 ~/.ssh/guardlinkx-key califaxvpn.guardlinkx.com admin@guardlinkx.com"
    exit 1
fi

SERVER=$1
KEY=$2
DOMAIN=${3:-califaxvpn.guardlinkx.com}
EMAIL=${4:-admin@guardlinkx.com}

echo "=== Setting up SSL for $DOMAIN ==="
echo "Server: $SERVER"
echo "Email: $EMAIL"
echo ""

ssh -i "$KEY" -o StrictHostKeyChecking=no "ubuntu@$SERVER" << REMOTE_SCRIPT
    set -euo pipefail

    echo "Running Certbot for $DOMAIN..."
    sudo certbot --nginx \
        -d "$DOMAIN" \
        --non-interactive \
        --agree-tos \
        -m "$EMAIL" \
        --redirect

    echo ""
    echo "=== SSL setup complete! ==="
    echo "Certificate installed for: $DOMAIN"
    echo ""
    echo "Certbot auto-renewal is enabled by default."
    echo "Test renewal with: sudo certbot renew --dry-run"

    # Verify Nginx config
    sudo nginx -t && sudo systemctl reload nginx
REMOTE_SCRIPT

echo ""
echo "=== Done! Visit https://$DOMAIN ==="

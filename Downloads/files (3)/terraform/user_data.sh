#!/bin/bash
# GuardLinkX Storefront — EC2 Bootstrap Script
# This runs as root on first boot via cloud-init.
# NOTE: Processed by Terraform templatefile().
set -euo pipefail
exec > /var/log/guardlinkx-setup.log 2>&1

echo "=== GuardLinkX setup starting ==="

# --------------------------------------------------------------------
# 1. System packages
# --------------------------------------------------------------------
export DEBIAN_FRONTEND=noninteractive
apt-get update -y
apt-get upgrade -y

# Python 3.12 from deadsnakes PPA
apt-get install -y software-properties-common
add-apt-repository -y ppa:deadsnakes/ppa
apt-get update -y

apt-get install -y \
  python3.12 python3.12-venv python3.12-dev \
  nginx \
  postgresql postgresql-contrib \
  certbot python3-certbot-nginx \
  rsync curl git libpq-dev

# --------------------------------------------------------------------
# 2. Create system user
# --------------------------------------------------------------------
useradd --system --shell /usr/sbin/nologin --home /opt/guardlinkx guardlinkx || true

# --------------------------------------------------------------------
# 3. PostgreSQL database & user
# --------------------------------------------------------------------
systemctl enable postgresql
systemctl start postgresql

sudo -u postgres psql -c "CREATE USER guardlinkx WITH PASSWORD '${db_password}';" || true
sudo -u postgres psql -c "CREATE DATABASE guardlinkx OWNER guardlinkx;" || true

# --------------------------------------------------------------------
# 4. Application directory & venv
# --------------------------------------------------------------------
mkdir -p /opt/guardlinkx/app
python3.12 -m venv /opt/guardlinkx/venv
chown -R guardlinkx:guardlinkx /opt/guardlinkx

# --------------------------------------------------------------------
# 5. Environment file
# --------------------------------------------------------------------
cat > /opt/guardlinkx/.env << 'ENVEOF'
FLASK_ENV=production
SECRET_KEY=${flask_secret_key}
DATABASE_URL=postgresql://guardlinkx:${db_password}@localhost:5432/guardlinkx
STRIPE_SECRET_KEY=${stripe_secret_key}
STRIPE_PUBLISHABLE_KEY=${stripe_publishable_key}
STRIPE_WEBHOOK_SECRET=${stripe_webhook_secret}
ADMIN_EMAIL=${admin_email}
ADMIN_PASSWORD=${admin_password}
DOMAIN=${domain_name}
ENVEOF
chmod 600 /opt/guardlinkx/.env
chown guardlinkx:guardlinkx /opt/guardlinkx/.env

# --------------------------------------------------------------------
# 6. Nginx configuration
# Nginx reverse proxy to Gunicorn socket
# --------------------------------------------------------------------
cat > /etc/nginx/sites-available/guardlinkx << 'NGINXEOF'
server {
    listen 80;
    server_name ${domain_name};

    location / {
        proxy_pass http://unix:/opt/guardlinkx/gunicorn.sock;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Allow large file uploads (trial downloads, etc.)
    client_max_body_size 200M;
}
NGINXEOF

ln -sf /etc/nginx/sites-available/guardlinkx /etc/nginx/sites-enabled/guardlinkx
rm -f /etc/nginx/sites-enabled/default
nginx -t
systemctl enable nginx
systemctl reload nginx

# --------------------------------------------------------------------
# 7. Gunicorn systemd service
# --------------------------------------------------------------------
cat > /etc/systemd/system/guardlinkx.service << 'SVCEOF'
[Unit]
Description=GuardLinkX Flask Storefront
After=network.target postgresql.service
Requires=postgresql.service

[Service]
User=guardlinkx
Group=guardlinkx
WorkingDirectory=/opt/guardlinkx/app
EnvironmentFile=/opt/guardlinkx/.env
ExecStart=/opt/guardlinkx/venv/bin/gunicorn \
    --workers 3 \
    --bind unix:/opt/guardlinkx/gunicorn.sock \
    --timeout 120 \
    --access-logfile /opt/guardlinkx/access.log \
    --error-logfile /opt/guardlinkx/error.log \
    "app:create_app()"
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
SVCEOF

systemctl daemon-reload
systemctl enable guardlinkx

# --------------------------------------------------------------------
# 8. Set up log rotation
# --------------------------------------------------------------------
cat > /etc/logrotate.d/guardlinkx << 'LOGEOF'
/opt/guardlinkx/*.log {
    daily
    rotate 14
    compress
    missingok
    notifempty
    postrotate
        systemctl reload guardlinkx > /dev/null 2>&1 || true
    endscript
}
LOGEOF

echo "=== GuardLinkX setup completed ==="
echo "Run deploy.sh to upload the app code, then start the service."

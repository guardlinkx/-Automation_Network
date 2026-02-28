import uuid
from datetime import datetime, timedelta, timezone
from flask_sqlalchemy import SQLAlchemy
from flask_login import UserMixin
from werkzeug.security import generate_password_hash, check_password_hash

db = SQLAlchemy()


class AdminUser(UserMixin, db.Model):
    __tablename__ = "admin_users"

    id = db.Column(db.Integer, primary_key=True)
    email = db.Column(db.String(255), unique=True, nullable=False)
    password_hash = db.Column(db.String(255), nullable=False)
    name = db.Column(db.String(100), default="Admin")
    is_active_user = db.Column(db.Boolean, default=True)
    created_at = db.Column(db.DateTime, default=lambda: datetime.now(timezone.utc))

    def set_password(self, password):
        self.password_hash = generate_password_hash(password)

    def check_password(self, password):
        return check_password_hash(self.password_hash, password)


class Product(db.Model):
    __tablename__ = "products"

    id = db.Column(db.Integer, primary_key=True)
    slug = db.Column(db.String(100), unique=True, nullable=False)
    name = db.Column(db.String(200), nullable=False)
    tagline = db.Column(db.String(300), default="")
    description = db.Column(db.Text, default="")
    icon = db.Column(db.String(10), default="🛡️")
    color = db.Column(db.String(7), default="#00E5A0")
    category = db.Column(db.String(50), default="software")  # software, mobile, vpn
    is_active = db.Column(db.Boolean, default=True)
    sort_order = db.Column(db.Integer, default=0)
    created_at = db.Column(db.DateTime, default=lambda: datetime.now(timezone.utc))
    updated_at = db.Column(
        db.DateTime,
        default=lambda: datetime.now(timezone.utc),
        onupdate=lambda: datetime.now(timezone.utc),
    )

    features = db.relationship(
        "ProductFeature", backref="product", lazy=True, cascade="all, delete-orphan",
        order_by="ProductFeature.sort_order"
    )
    plans = db.relationship(
        "Plan", backref="product", lazy=True, cascade="all, delete-orphan",
        order_by="Plan.sort_order"
    )

    def to_dict(self):
        return {
            "id": self.id,
            "slug": self.slug,
            "name": self.name,
            "tagline": self.tagline,
            "description": self.description,
            "icon": self.icon,
            "color": self.color,
            "category": self.category,
            "is_active": self.is_active,
            "sort_order": self.sort_order,
            "features": [f.text for f in self.features],
            "plans": [p.to_dict() for p in self.plans],
        }


class ProductFeature(db.Model):
    __tablename__ = "product_features"

    id = db.Column(db.Integer, primary_key=True)
    product_id = db.Column(db.Integer, db.ForeignKey("products.id"), nullable=False)
    text = db.Column(db.String(300), nullable=False)
    sort_order = db.Column(db.Integer, default=0)


class Plan(db.Model):
    __tablename__ = "plans"

    id = db.Column(db.Integer, primary_key=True)
    product_id = db.Column(db.Integer, db.ForeignKey("products.id"), nullable=False)
    name = db.Column(db.String(200), nullable=False)
    price_display = db.Column(db.String(50), nullable=False)  # e.g. "$4.99"
    period = db.Column(db.String(50), default="/mo")  # e.g. "/mo", " one-time"
    annual_display = db.Column(db.String(100), default="")  # e.g. "$49.99/yr"
    annual_save_text = db.Column(db.String(50), default="")  # e.g. "Save 17%"
    trial_text = db.Column(db.String(100), default="")  # e.g. "7-day free trial"
    badge = db.Column(db.String(50), default="")  # e.g. "MOST POPULAR"
    btn_text = db.Column(db.String(50), default="Start Free Trial")
    btn_url = db.Column(db.String(500), default="")  # Stripe checkout URL or external link
    price_type = db.Column(db.String(20), default="subscription")  # subscription, one_time, custom, paygo
    price_cents = db.Column(db.Integer, default=0)  # actual price in cents for Stripe
    stripe_price_id = db.Column(db.String(200), default="")  # Stripe Price ID
    stripe_product_id = db.Column(db.String(200), default="")  # Stripe Product ID
    paygo_rate = db.Column(db.String(100), default="")  # for pay-as-you-go display
    is_active = db.Column(db.Boolean, default=True)
    sort_order = db.Column(db.Integer, default=0)

    plan_features = db.relationship(
        "PlanFeature", backref="plan", lazy=True, cascade="all, delete-orphan",
        order_by="PlanFeature.sort_order"
    )

    def to_dict(self):
        return {
            "id": self.id,
            "name": self.name,
            "price_display": self.price_display,
            "period": self.period,
            "annual_display": self.annual_display,
            "annual_save_text": self.annual_save_text,
            "trial_text": self.trial_text,
            "badge": self.badge,
            "btn_text": self.btn_text,
            "btn_url": self.btn_url,
            "price_type": self.price_type,
            "price_cents": self.price_cents,
            "stripe_price_id": self.stripe_price_id,
            "paygo_rate": self.paygo_rate,
            "is_active": self.is_active,
            "features": [f.text for f in self.plan_features],
        }


class PlanFeature(db.Model):
    __tablename__ = "plan_features"

    id = db.Column(db.Integer, primary_key=True)
    plan_id = db.Column(db.Integer, db.ForeignKey("plans.id"), nullable=False)
    text = db.Column(db.String(300), nullable=False)
    sort_order = db.Column(db.Integer, default=0)


class TrialRegistration(db.Model):
    __tablename__ = "trial_registrations"

    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String(200), nullable=False)
    email = db.Column(db.String(255), nullable=False)
    product_slug = db.Column(db.String(100), nullable=False)
    created_at = db.Column(db.DateTime, default=lambda: datetime.now(timezone.utc))
    download_token = db.Column(
        db.String(36),
        unique=True,
        nullable=False,
        default=lambda: str(uuid.uuid4()),
    )
    token_expires_at = db.Column(
        db.DateTime,
        nullable=False,
        default=lambda: datetime.now(timezone.utc) + timedelta(hours=24),
    )
    downloaded = db.Column(db.Boolean, default=False)


class Order(db.Model):
    __tablename__ = "orders"

    id = db.Column(db.Integer, primary_key=True)
    stripe_session_id = db.Column(db.String(300), unique=True)
    stripe_customer_id = db.Column(db.String(200), default="")
    stripe_subscription_id = db.Column(db.String(200), default="")
    customer_email = db.Column(db.String(255), default="")
    customer_name = db.Column(db.String(200), default="")
    plan_id = db.Column(db.Integer, db.ForeignKey("plans.id"), nullable=True)
    plan_name = db.Column(db.String(200), default="")
    product_name = db.Column(db.String(200), default="")
    amount_cents = db.Column(db.Integer, default=0)
    currency = db.Column(db.String(3), default="usd")
    status = db.Column(db.String(50), default="pending")  # pending, completed, failed, refunded
    payment_type = db.Column(db.String(20), default="subscription")  # subscription, one_time
    created_at = db.Column(db.DateTime, default=lambda: datetime.now(timezone.utc))
    completed_at = db.Column(db.DateTime, nullable=True)

    plan = db.relationship("Plan", backref="orders")

    def to_dict(self):
        return {
            "id": self.id,
            "customer_email": self.customer_email,
            "customer_name": self.customer_name,
            "plan_name": self.plan_name,
            "product_name": self.product_name,
            "amount": f"${self.amount_cents / 100:.2f}",
            "status": self.status,
            "payment_type": self.payment_type,
            "created_at": self.created_at.strftime("%Y-%m-%d %H:%M") if self.created_at else "",
        }


class VpnServer(db.Model):
    __tablename__ = "vpn_servers"

    id = db.Column(db.Integer, primary_key=True)
    region = db.Column(db.String(30), nullable=False)        # e.g. us-east-1
    country = db.Column(db.String(100), nullable=False)      # e.g. United States
    city = db.Column(db.String(100), nullable=False)         # e.g. Virginia
    ip_address = db.Column(db.String(45), unique=True, nullable=False)
    api_port = db.Column(db.Integer, default=8443)
    node_secret = db.Column(db.String(255), nullable=False)
    is_active = db.Column(db.Boolean, default=True)
    max_peers = db.Column(db.Integer, default=250)
    created_at = db.Column(db.DateTime, default=lambda: datetime.now(timezone.utc))

    sessions = db.relationship(
        "VpnSession", backref="server", lazy=True, cascade="all, delete-orphan"
    )

    def to_dict(self, include_secret=False):
        d = {
            "id": self.id,
            "region": self.region,
            "country": self.country,
            "city": self.city,
            "ip_address": self.ip_address,
            "is_active": self.is_active,
            "max_peers": self.max_peers,
        }
        if include_secret:
            d["node_secret"] = self.node_secret
            d["api_port"] = self.api_port
        return d


class MeshNode(db.Model):
    __tablename__ = "mesh_nodes"

    id = db.Column(db.Integer, primary_key=True)
    peer_id = db.Column(db.String(64), unique=True, nullable=False)
    public_key = db.Column(db.String(255), nullable=False)
    endpoint = db.Column(db.String(255), nullable=False)
    region = db.Column(db.String(30), nullable=False)
    country = db.Column(db.String(100), default="")
    bandwidth_mbps = db.Column(db.Integer, default=100)
    is_relay = db.Column(db.Boolean, default=True)
    is_exit = db.Column(db.Boolean, default=False)
    is_active = db.Column(db.Boolean, default=True)
    last_seen = db.Column(db.DateTime, default=lambda: datetime.now(timezone.utc))
    created_at = db.Column(db.DateTime, default=lambda: datetime.now(timezone.utc))

    def to_dict(self):
        return {
            "id": self.id,
            "peer_id": self.peer_id,
            "public_key": self.public_key,
            "endpoint": self.endpoint,
            "region": self.region,
            "country": self.country,
            "bandwidth_mbps": self.bandwidth_mbps,
            "is_relay": self.is_relay,
            "is_exit": self.is_exit,
            "is_active": self.is_active,
            "last_seen": self.last_seen.isoformat() if self.last_seen else None,
        }


class MeshCircuit(db.Model):
    __tablename__ = "mesh_circuits"

    id = db.Column(db.Integer, primary_key=True)
    circuit_id = db.Column(db.String(64), unique=True, nullable=False)
    hop_count = db.Column(db.Integer, nullable=False)
    node_ids = db.Column(db.Text, nullable=False)  # comma-separated node IDs
    entry_node_id = db.Column(db.Integer, nullable=True)
    exit_node_id = db.Column(db.Integer, nullable=True)
    status = db.Column(db.String(20), default="active")  # active, reshuffled, destroyed
    created_at = db.Column(db.DateTime, default=lambda: datetime.now(timezone.utc))
    reshuffled_at = db.Column(db.DateTime, nullable=True)

    def to_dict(self):
        return {
            "id": self.id,
            "circuit_id": self.circuit_id,
            "hop_count": self.hop_count,
            "node_ids": self.node_ids,
            "entry_node_id": self.entry_node_id,
            "exit_node_id": self.exit_node_id,
            "status": self.status,
            "created_at": self.created_at.isoformat() if self.created_at else None,
        }


class BlockchainIdentity(db.Model):
    __tablename__ = "blockchain_identities"

    id = db.Column(db.Integer, primary_key=True)
    wallet_address = db.Column(db.String(42), unique=True, nullable=False)
    public_key = db.Column(db.String(255), nullable=False)
    did = db.Column(db.String(100), unique=True, nullable=False)
    trust_score = db.Column(db.Integer, default=100)
    is_active = db.Column(db.Boolean, default=True)
    blockchain_tx = db.Column(db.String(66), nullable=True)
    created_at = db.Column(db.DateTime, default=lambda: datetime.now(timezone.utc))

    def to_dict(self):
        return {
            "id": self.id,
            "wallet_address": self.wallet_address,
            "did": self.did,
            "trust_score": self.trust_score,
            "is_active": self.is_active,
            "blockchain_tx": self.blockchain_tx,
            "created_at": self.created_at.isoformat() if self.created_at else None,
        }


class LocationPolicy(db.Model):
    __tablename__ = "location_policies"

    id = db.Column(db.Integer, primary_key=True)
    device_id = db.Column(db.String(255), nullable=False, index=True)
    policy_type = db.Column(db.String(30), nullable=False)  # gps_spoof, location_fuzz, wifi_mask, etc.
    spoof_latitude = db.Column(db.Float, nullable=True)
    spoof_longitude = db.Column(db.Float, nullable=True)
    fuzz_radius_meters = db.Column(db.Integer, default=1000)
    target_apps = db.Column(db.Text, default="")
    config_json = db.Column(db.JSON, default=dict)
    is_active = db.Column(db.Boolean, default=True)
    created_at = db.Column(db.DateTime, default=lambda: datetime.now(timezone.utc))

    def to_dict(self):
        return {
            "id": self.id,
            "device_id": self.device_id,
            "policy_type": self.policy_type,
            "spoof_latitude": self.spoof_latitude,
            "spoof_longitude": self.spoof_longitude,
            "fuzz_radius_meters": self.fuzz_radius_meters,
            "target_apps": self.target_apps,
            "config": self.config_json,
            "is_active": self.is_active,
        }


class ThreatEvent(db.Model):
    __tablename__ = "threat_events"

    id = db.Column(db.Integer, primary_key=True)
    event_type = db.Column(db.String(50), nullable=False)  # ddos, portscan, exfil, anomaly
    severity = db.Column(db.String(20), default="medium")  # low, medium, high, critical
    source_ip = db.Column(db.String(45), default="")
    description = db.Column(db.Text, default="")
    metadata_json = db.Column(db.JSON, default=dict)
    detected_at = db.Column(db.DateTime, default=lambda: datetime.now(timezone.utc))

    def to_dict(self):
        return {
            "id": self.id,
            "event_type": self.event_type,
            "severity": self.severity,
            "source_ip": self.source_ip,
            "description": self.description,
            "metadata": self.metadata_json,
            "detected_at": self.detected_at.isoformat() if self.detected_at else None,
        }


class VpnSession(db.Model):
    __tablename__ = "vpn_sessions"

    id = db.Column(db.Integer, primary_key=True)
    license_key = db.Column(db.String(255), nullable=False, index=True)
    device_id = db.Column(db.String(255), nullable=False)
    server_id = db.Column(db.Integer, db.ForeignKey("vpn_servers.id"), nullable=False)
    client_public_key = db.Column(db.String(44), nullable=False)
    client_ip = db.Column(db.String(20), nullable=False)
    status = db.Column(db.String(20), default="active")  # active, disconnected
    connected_at = db.Column(db.DateTime, default=lambda: datetime.now(timezone.utc))
    disconnected_at = db.Column(db.DateTime, nullable=True)

    def to_dict(self):
        return {
            "id": self.id,
            "license_key": self.license_key,
            "device_id": self.device_id,
            "server_id": self.server_id,
            "client_ip": self.client_ip,
            "status": self.status,
            "connected_at": self.connected_at.isoformat() if self.connected_at else None,
            "disconnected_at": self.disconnected_at.isoformat() if self.disconnected_at else None,
        }

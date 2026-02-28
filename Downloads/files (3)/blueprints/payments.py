import stripe
from datetime import datetime, timezone
from flask import Blueprint, request, redirect, jsonify, render_template, current_app
from models import db, Plan, Order

payments_bp = Blueprint("payments", __name__, url_prefix="/payments")


def get_stripe():
    stripe.api_key = current_app.config["STRIPE_SECRET_KEY"]
    return stripe


# ── Checkout ──────────────────────────────────────────────────────────
@payments_bp.route("/checkout/<int:plan_id>", methods=["POST"])
def create_checkout(plan_id):
    """Create a Stripe Checkout Session for a plan."""
    s = get_stripe()
    plan = Plan.query.get_or_404(plan_id)
    domain = current_app.config["DOMAIN"]
    base_url = f"https://{domain}"

    # Build line items
    if plan.stripe_price_id:
        # Use existing Stripe Price
        line_items = [{"price": plan.stripe_price_id, "quantity": 1}]
    else:
        # Create price on the fly
        line_item = {
            "price_data": {
                "currency": "usd",
                "product_data": {"name": f"{plan.product.name} — {plan.name}"},
                "unit_amount": plan.price_cents,
            },
            "quantity": 1,
        }
        if plan.price_type == "subscription":
            line_item["price_data"]["recurring"] = {"interval": "month"}
        line_items = [line_item]

    # Session params
    params = {
        "payment_method_types": ["card"],
        "line_items": line_items,
        "mode": "subscription" if plan.price_type == "subscription" else "payment",
        "success_url": f"{base_url}/payments/success?session_id={{CHECKOUT_SESSION_ID}}",
        "cancel_url": f"{base_url}/payments/cancel",
        "metadata": {"plan_id": str(plan.id), "product_name": plan.product.name},
    }

    # Add trial if plan has one
    if plan.trial_text and plan.price_type == "subscription":
        # Extract trial days from text like "7-day free trial"
        trial_days = 7  # default
        parts = plan.trial_text.split("-")
        if parts and parts[0].strip().isdigit():
            trial_days = int(parts[0].strip())
        params["subscription_data"] = {"trial_period_days": trial_days}

    try:
        session = s.checkout.Session.create(**params)

        # Record the order
        order = Order(
            stripe_session_id=session.id,
            plan_id=plan.id,
            plan_name=plan.name,
            product_name=plan.product.name,
            amount_cents=plan.price_cents,
            payment_type=plan.price_type,
            status="pending",
        )
        db.session.add(order)
        db.session.commit()

        return redirect(session.url)
    except Exception as e:
        current_app.logger.error(f"Stripe checkout error: {e}")
        return jsonify({"error": str(e)}), 400


# ── Success / Cancel pages ────────────────────────────────────────────
@payments_bp.route("/success")
def success():
    session_id = request.args.get("session_id")
    order = None
    if session_id:
        order = Order.query.filter_by(stripe_session_id=session_id).first()
        if order and order.status == "pending":
            try:
                s = get_stripe()
                session = s.checkout.Session.retrieve(session_id)
                order.customer_email = session.customer_details.email or ""
                order.customer_name = session.customer_details.name or ""
                order.stripe_customer_id = session.customer or ""
                if session.subscription:
                    order.stripe_subscription_id = session.subscription
                order.status = "completed"
                order.completed_at = datetime.now(timezone.utc)
                db.session.commit()
            except Exception as e:
                current_app.logger.error(f"Error retrieving session: {e}")
    return render_template("checkout/success.html", order=order)


@payments_bp.route("/cancel")
def cancel():
    return render_template("checkout/cancel.html")


# ── Stripe Webhook ────────────────────────────────────────────────────
@payments_bp.route("/webhook", methods=["POST"])
def webhook():
    """Handle Stripe webhook events for payment confirmations."""
    s = get_stripe()
    payload = request.get_data()
    sig = request.headers.get("Stripe-Signature")
    webhook_secret = current_app.config["STRIPE_WEBHOOK_SECRET"]

    try:
        event = s.Webhook.construct_event(payload, sig, webhook_secret)
    except ValueError:
        return "Invalid payload", 400
    except s.error.SignatureVerificationError:
        return "Invalid signature", 400

    if event["type"] == "checkout.session.completed":
        session = event["data"]["object"]
        order = Order.query.filter_by(stripe_session_id=session["id"]).first()
        if order:
            order.status = "completed"
            order.customer_email = session.get("customer_details", {}).get("email", "")
            order.customer_name = session.get("customer_details", {}).get("name", "")
            order.stripe_customer_id = session.get("customer", "")
            order.completed_at = datetime.now(timezone.utc)
            if session.get("subscription"):
                order.stripe_subscription_id = session["subscription"]
            db.session.commit()

    elif event["type"] == "invoice.payment_failed":
        invoice = event["data"]["object"]
        sub_id = invoice.get("subscription")
        if sub_id:
            order = Order.query.filter_by(stripe_subscription_id=sub_id).first()
            if order:
                order.status = "failed"
                db.session.commit()

    return "OK", 200

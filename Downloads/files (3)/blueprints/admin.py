from flask import Blueprint, render_template, request, redirect, url_for, flash, jsonify
from flask_login import login_user, logout_user, login_required, current_user
from models import db, AdminUser, Product, ProductFeature, Plan, PlanFeature, Order, TrialRegistration

admin_bp = Blueprint("admin", __name__, url_prefix="/admin")


# ── Auth ──────────────────────────────────────────────────────────────
@admin_bp.route("/login", methods=["GET", "POST"])
def login():
    if current_user.is_authenticated:
        return redirect(url_for("admin.dashboard"))
    if request.method == "POST":
        email = request.form.get("email", "").strip()
        password = request.form.get("password", "")
        user = AdminUser.query.filter_by(email=email).first()
        if user and user.check_password(password):
            login_user(user)
            return redirect(url_for("admin.dashboard"))
        flash("Invalid credentials", "error")
    return render_template("admin/login.html")


@admin_bp.route("/logout")
@login_required
def logout():
    logout_user()
    return redirect(url_for("admin.login"))


# ── Dashboard ─────────────────────────────────────────────────────────
@admin_bp.route("/")
@login_required
def dashboard():
    products = Product.query.order_by(Product.sort_order).all()
    orders = Order.query.order_by(Order.created_at.desc()).limit(20).all()
    stats = {
        "total_products": Product.query.filter_by(is_active=True).count(),
        "total_plans": Plan.query.filter_by(is_active=True).count(),
        "total_orders": Order.query.count(),
        "completed_orders": Order.query.filter_by(status="completed").count(),
    }
    return render_template("admin/dashboard.html", products=products, orders=orders, stats=stats)


# ── Products CRUD ─────────────────────────────────────────────────────
@admin_bp.route("/products")
@login_required
def products():
    products = Product.query.order_by(Product.sort_order).all()
    return render_template("admin/products.html", products=products)


@admin_bp.route("/products/new", methods=["GET", "POST"])
@login_required
def product_new():
    if request.method == "POST":
        product = Product(
            name=request.form["name"],
            slug=request.form["slug"],
            tagline=request.form.get("tagline", ""),
            description=request.form.get("description", ""),
            icon=request.form.get("icon", "🛡️"),
            color=request.form.get("color", "#00E5A0"),
            category=request.form.get("category", "software"),
            sort_order=int(request.form.get("sort_order", 0)),
            is_active="is_active" in request.form,
        )
        # Parse features (one per line)
        features_text = request.form.get("features_text", "")
        for i, line in enumerate(features_text.strip().split("\n")):
            line = line.strip()
            if line:
                product.features.append(ProductFeature(text=line, sort_order=i))

        db.session.add(product)
        db.session.commit()
        flash(f"Product '{product.name}' created!", "success")
        return redirect(url_for("admin.products"))
    return render_template("admin/product_edit.html", product=None)


@admin_bp.route("/products/<int:product_id>/edit", methods=["GET", "POST"])
@login_required
def product_edit(product_id):
    product = Product.query.get_or_404(product_id)
    if request.method == "POST":
        product.name = request.form["name"]
        product.slug = request.form["slug"]
        product.tagline = request.form.get("tagline", "")
        product.description = request.form.get("description", "")
        product.icon = request.form.get("icon", "🛡️")
        product.color = request.form.get("color", "#00E5A0")
        product.category = request.form.get("category", "software")
        product.sort_order = int(request.form.get("sort_order", 0))
        product.is_active = "is_active" in request.form

        # Replace features
        ProductFeature.query.filter_by(product_id=product.id).delete()
        features_text = request.form.get("features_text", "")
        for i, line in enumerate(features_text.strip().split("\n")):
            line = line.strip()
            if line:
                db.session.add(ProductFeature(product_id=product.id, text=line, sort_order=i))

        db.session.commit()
        flash(f"Product '{product.name}' updated!", "success")
        return redirect(url_for("admin.products"))
    return render_template("admin/product_edit.html", product=product)


@admin_bp.route("/products/<int:product_id>/delete", methods=["POST"])
@login_required
def product_delete(product_id):
    product = Product.query.get_or_404(product_id)
    name = product.name
    db.session.delete(product)
    db.session.commit()
    flash(f"Product '{name}' deleted.", "success")
    return redirect(url_for("admin.products"))


@admin_bp.route("/products/<int:product_id>/toggle", methods=["POST"])
@login_required
def product_toggle(product_id):
    product = Product.query.get_or_404(product_id)
    product.is_active = not product.is_active
    db.session.commit()
    return jsonify({"is_active": product.is_active})


# ── Plans CRUD ────────────────────────────────────────────────────────
@admin_bp.route("/products/<int:product_id>/plans")
@login_required
def plans(product_id):
    product = Product.query.get_or_404(product_id)
    return render_template("admin/plans.html", product=product)


@admin_bp.route("/products/<int:product_id>/plans/new", methods=["GET", "POST"])
@login_required
def plan_new(product_id):
    product = Product.query.get_or_404(product_id)
    if request.method == "POST":
        plan = Plan(
            product_id=product.id,
            name=request.form["name"],
            price_display=request.form["price_display"],
            period=request.form.get("period", "/mo"),
            annual_display=request.form.get("annual_display", ""),
            annual_save_text=request.form.get("annual_save_text", ""),
            trial_text=request.form.get("trial_text", ""),
            badge=request.form.get("badge", ""),
            btn_text=request.form.get("btn_text", "Start Free Trial"),
            btn_url=request.form.get("btn_url", ""),
            price_type=request.form.get("price_type", "subscription"),
            price_cents=int(request.form.get("price_cents", 0)),
            stripe_price_id=request.form.get("stripe_price_id", ""),
            stripe_product_id=request.form.get("stripe_product_id", ""),
            paygo_rate=request.form.get("paygo_rate", ""),
            sort_order=int(request.form.get("sort_order", 0)),
            is_active="is_active" in request.form,
        )
        features_text = request.form.get("features_text", "")
        for i, line in enumerate(features_text.strip().split("\n")):
            line = line.strip()
            if line:
                plan.plan_features.append(PlanFeature(text=line, sort_order=i))

        db.session.add(plan)
        db.session.commit()
        flash(f"Plan '{plan.name}' created!", "success")
        return redirect(url_for("admin.plans", product_id=product.id))
    return render_template("admin/plan_edit.html", product=product, plan=None)


@admin_bp.route("/plans/<int:plan_id>/edit", methods=["GET", "POST"])
@login_required
def plan_edit(plan_id):
    plan = Plan.query.get_or_404(plan_id)
    product = plan.product
    if request.method == "POST":
        plan.name = request.form["name"]
        plan.price_display = request.form["price_display"]
        plan.period = request.form.get("period", "/mo")
        plan.annual_display = request.form.get("annual_display", "")
        plan.annual_save_text = request.form.get("annual_save_text", "")
        plan.trial_text = request.form.get("trial_text", "")
        plan.badge = request.form.get("badge", "")
        plan.btn_text = request.form.get("btn_text", "Start Free Trial")
        plan.btn_url = request.form.get("btn_url", "")
        plan.price_type = request.form.get("price_type", "subscription")
        plan.price_cents = int(request.form.get("price_cents", 0))
        plan.stripe_price_id = request.form.get("stripe_price_id", "")
        plan.stripe_product_id = request.form.get("stripe_product_id", "")
        plan.paygo_rate = request.form.get("paygo_rate", "")
        plan.sort_order = int(request.form.get("sort_order", 0))
        plan.is_active = "is_active" in request.form

        PlanFeature.query.filter_by(plan_id=plan.id).delete()
        features_text = request.form.get("features_text", "")
        for i, line in enumerate(features_text.strip().split("\n")):
            line = line.strip()
            if line:
                db.session.add(PlanFeature(plan_id=plan.id, text=line, sort_order=i))

        db.session.commit()
        flash(f"Plan '{plan.name}' updated!", "success")
        return redirect(url_for("admin.plans", product_id=product.id))
    return render_template("admin/plan_edit.html", product=product, plan=plan)


@admin_bp.route("/plans/<int:plan_id>/delete", methods=["POST"])
@login_required
def plan_delete(plan_id):
    plan = Plan.query.get_or_404(plan_id)
    product_id = plan.product_id
    name = plan.name
    db.session.delete(plan)
    db.session.commit()
    flash(f"Plan '{name}' deleted.", "success")
    return redirect(url_for("admin.plans", product_id=product_id))


# ── Trial Registrations ──────────────────────────────────────────────
@admin_bp.route("/trials")
@login_required
def trials():
    page = request.args.get("page", 1, type=int)
    pagination = (
        TrialRegistration.query
        .order_by(TrialRegistration.created_at.desc())
        .paginate(page=page, per_page=25)
    )
    return render_template("admin/trials.html", pagination=pagination)


# ── Orders ────────────────────────────────────────────────────────────
@admin_bp.route("/orders")
@login_required
def orders():
    page = request.args.get("page", 1, type=int)
    status_filter = request.args.get("status", "")
    query = Order.query
    if status_filter:
        query = query.filter_by(status=status_filter)
    pagination = query.order_by(Order.created_at.desc()).paginate(page=page, per_page=25)
    return render_template("admin/orders.html", pagination=pagination, status_filter=status_filter)

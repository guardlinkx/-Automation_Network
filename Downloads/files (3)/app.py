from flask import Flask, render_template
from flask_login import LoginManager
from flask_migrate import Migrate
from config import Config
from models import db, AdminUser, Product


def create_app():
    app = Flask(__name__)
    app.config.from_object(Config)

    # Initialize extensions
    db.init_app(app)
    Migrate(app, db)

    login_manager = LoginManager()
    login_manager.init_app(app)
    login_manager.login_view = "admin.login"
    login_manager.login_message_category = "info"

    @login_manager.user_loader
    def load_user(user_id):
        return AdminUser.query.get(int(user_id))
    

    # Register blueprints
    from blueprints.admin import admin_bp
    from blueprints.api import api_bp
    from blueprints.payments import payments_bp
    from blueprints.downloads import downloads_bp
    from blueprints.vpn import vpn_bp
    from blueprints.ai_intelligence import ai_bp
    from blueprints.mesh import mesh_bp
    from blueprints.zero_trust import zt_bp
    from blueprints.location import location_bp

    app.register_blueprint(admin_bp)
    app.register_blueprint(api_bp)
    app.register_blueprint(payments_bp)
    app.register_blueprint(downloads_bp)
    app.register_blueprint(vpn_bp)
    app.register_blueprint(ai_bp)
    app.register_blueprint(mesh_bp)
    app.register_blueprint(zt_bp)
    app.register_blueprint(location_bp)

    # Public storefront
    @app.route("/")
    def index():
        products = (
            Product.query
            .filter_by(is_active=True)
            .order_by(Product.sort_order)
            .all()
        )
        return render_template(
            "index.html",
            products=products,
            stripe_key=app.config["STRIPE_PUBLISHABLE_KEY"],
        )

    # Create tables and default admin on first run
    with app.app_context():
        db.create_all()
        if not AdminUser.query.first():
            admin = AdminUser(
                email=app.config["ADMIN_EMAIL"],
                name="Administrator",
            )
            admin.set_password(app.config["ADMIN_PASSWORD"])
            db.session.add(admin)
            db.session.commit()
            print(f"[+] Admin user created: {admin.email}")

    return app


app = create_app()

if __name__ == "__main__":
    app.run(debug=True, host="0.0.0.0", port=5050)

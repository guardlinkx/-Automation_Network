from flask import Blueprint, jsonify
from models import Product

api_bp = Blueprint("api", __name__, url_prefix="/api")


@api_bp.route("/products")
def get_products():
    """Return all active products with plans and features for the storefront."""
    products = (
        Product.query
        .filter_by(is_active=True)
        .order_by(Product.sort_order)
        .all()
    )
    return jsonify([p.to_dict() for p in products])


@api_bp.route("/products/<slug>")
def get_product(slug):
    """Return a single product by slug."""
    product = Product.query.filter_by(slug=slug, is_active=True).first_or_404()
    return jsonify(product.to_dict())

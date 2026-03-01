"""
Utility module for exporting trained ML models to ONNX format.

Provides helper functions for scikit-learn, XGBoost, and LightGBM models,
plus validation of exported ONNX files.
"""

import os
import numpy as np
import onnx
import onnxruntime as ort
from skl2onnx import convert_sklearn
from skl2onnx.common.data_types import FloatTensorType
from onnxmltools import convert_xgboost, convert_lightgbm
from onnxmltools.convert.common.data_types import FloatTensorType as OnnxFloatTensorType


def export_sklearn_to_onnx(model, feature_names: list[str], output_path: str) -> str:
    """
    Export a scikit-learn model to ONNX format.

    Args:
        model: A fitted scikit-learn estimator.
        feature_names: List of input feature names.
        output_path: File path to save the .onnx model.

    Returns:
        The absolute path to the saved ONNX model.
    """
    n_features = len(feature_names)
    initial_type = [("input", FloatTensorType([None, n_features]))]

    onnx_model = convert_sklearn(
        model,
        initial_types=initial_type,
        target_opset={"": 15, "ai.onnx.ml": 3},
    )

    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    onnx.save_model(onnx_model, output_path)
    print(f"[export] Saved sklearn model to {output_path}")
    return os.path.abspath(output_path)


def export_xgboost_to_onnx(model, feature_names: list[str], output_path: str) -> str:
    """
    Export an XGBoost model to ONNX format.

    Args:
        model: A fitted XGBoost Booster or sklearn-API model.
        feature_names: List of input feature names.
        output_path: File path to save the .onnx model.

    Returns:
        The absolute path to the saved ONNX model.
    """
    n_features = len(feature_names)
    initial_type = [("input", OnnxFloatTensorType([None, n_features]))]

    onnx_model = convert_xgboost(
        model,
        initial_types=initial_type,
        target_opset=15,
    )

    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    onnx.save_model(onnx_model, output_path)
    print(f"[export] Saved XGBoost model to {output_path}")
    return os.path.abspath(output_path)


def export_lightgbm_to_onnx(model, feature_names: list[str], output_path: str) -> str:
    """
    Export a LightGBM model to ONNX format.

    Args:
        model: A fitted LightGBM Booster or sklearn-API model.
        feature_names: List of input feature names.
        output_path: File path to save the .onnx model.

    Returns:
        The absolute path to the saved ONNX model.
    """
    n_features = len(feature_names)
    initial_type = [("input", OnnxFloatTensorType([None, n_features]))]

    onnx_model = convert_lightgbm(
        model,
        initial_types=initial_type,
        target_opset=15,
    )

    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    onnx.save_model(onnx_model, output_path)
    print(f"[export] Saved LightGBM model to {output_path}")
    return os.path.abspath(output_path)


def validate_onnx(model_path: str) -> bool:
    """
    Load and validate an ONNX model file.

    Checks structural validity via onnx.checker and runs a dummy inference
    through onnxruntime to ensure the model is functional.

    Args:
        model_path: Path to the .onnx file.

    Returns:
        True if the model passes all checks, False otherwise.
    """
    try:
        # Structural check
        model = onnx.load(model_path)
        onnx.checker.check_model(model)
        print(f"[validate] ONNX checker passed for {model_path}")

        # Extract input shape for dummy inference
        session = ort.InferenceSession(model_path)
        input_meta = session.get_inputs()[0]
        input_name = input_meta.name
        input_shape = input_meta.shape

        # Replace dynamic axes (None or strings) with 1
        concrete_shape = [
            dim if isinstance(dim, int) and dim > 0 else 1
            for dim in input_shape
        ]

        dummy_input = np.random.randn(*concrete_shape).astype(np.float32)
        outputs = session.run(None, {input_name: dummy_input})

        print(f"[validate] Inference test passed. Output shapes: "
              f"{[o.shape for o in outputs]}")
        return True

    except onnx.checker.ValidationError as e:
        print(f"[validate] ONNX validation failed: {e}")
        return False
    except Exception as e:
        print(f"[validate] Runtime validation failed: {e}")
        return False

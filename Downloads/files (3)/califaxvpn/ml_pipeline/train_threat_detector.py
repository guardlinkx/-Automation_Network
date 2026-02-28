"""
Train an XGBoost binary classifier for network threat detection.

Generates synthetic training data mimicking CIC-IDS2017 feature distributions,
trains an XGBoost model, evaluates it, and exports to ONNX format.
"""

import os
import numpy as np
import pandas as pd
from sklearn.model_selection import train_test_split
from sklearn.metrics import accuracy_score, precision_score, recall_score, f1_score
from xgboost import XGBClassifier

from export_onnx import export_xgboost_to_onnx, validate_onnx

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
FEATURE_NAMES = [
    "packet_size",
    "flow_duration",
    "packet_rate",
    "byte_rate",
    "protocol_type",
    "port_entropy",
    "payload_entropy",
    "flag_syn",
    "flag_ack",
    "flag_rst",
    "flow_iat_mean",
    "flow_iat_std",
]

N_SAMPLES = 10_000
RANDOM_SEED = 42
MODEL_OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "models")
MODEL_OUTPUT_PATH = os.path.join(MODEL_OUTPUT_DIR, "threat_detector.onnx")


# ---------------------------------------------------------------------------
# Synthetic data generation
# ---------------------------------------------------------------------------
def generate_synthetic_data(n_samples: int, seed: int = RANDOM_SEED) -> pd.DataFrame:
    """
    Generate synthetic network-flow samples that approximate CIC-IDS2017
    feature distributions.

    Roughly 60 % normal traffic and 40 % attack traffic split across three
    attack categories: DDoS, port-scan, and data exfiltration.
    """
    rng = np.random.RandomState(seed)

    n_normal = int(n_samples * 0.6)
    n_ddos = int(n_samples * 0.15)
    n_portscan = int(n_samples * 0.15)
    n_exfil = n_samples - n_normal - n_ddos - n_portscan

    def _normal(n: int) -> dict:
        return {
            "packet_size": rng.normal(500, 200, n).clip(64, 1500),
            "flow_duration": rng.exponential(30, n).clip(0.1, 300),
            "packet_rate": rng.normal(50, 20, n).clip(1, 200),
            "byte_rate": rng.normal(25000, 10000, n).clip(100, 100000),
            "protocol_type": rng.choice([0, 1, 2], n, p=[0.5, 0.3, 0.2]),
            "port_entropy": rng.normal(1.5, 0.5, n).clip(0, 4),
            "payload_entropy": rng.normal(4.0, 1.0, n).clip(0, 8),
            "flag_syn": rng.binomial(1, 0.15, n),
            "flag_ack": rng.binomial(1, 0.80, n),
            "flag_rst": rng.binomial(1, 0.02, n),
            "flow_iat_mean": rng.normal(200, 80, n).clip(1, 1000),
            "flow_iat_std": rng.normal(100, 40, n).clip(0, 500),
            "label": np.zeros(n, dtype=int),
        }

    def _ddos(n: int) -> dict:
        return {
            "packet_size": rng.normal(200, 50, n).clip(64, 600),
            "flow_duration": rng.exponential(5, n).clip(0.01, 30),
            "packet_rate": rng.normal(5000, 2000, n).clip(500, 20000),
            "byte_rate": rng.normal(500000, 200000, n).clip(50000, 2000000),
            "protocol_type": rng.choice([0, 1, 2], n, p=[0.7, 0.2, 0.1]),
            "port_entropy": rng.normal(0.5, 0.3, n).clip(0, 2),
            "payload_entropy": rng.normal(2.0, 0.8, n).clip(0, 5),
            "flag_syn": rng.binomial(1, 0.85, n),
            "flag_ack": rng.binomial(1, 0.10, n),
            "flag_rst": rng.binomial(1, 0.30, n),
            "flow_iat_mean": rng.normal(5, 3, n).clip(0.1, 30),
            "flow_iat_std": rng.normal(2, 1, n).clip(0, 15),
            "label": np.ones(n, dtype=int),
        }

    def _portscan(n: int) -> dict:
        return {
            "packet_size": rng.normal(100, 30, n).clip(40, 300),
            "flow_duration": rng.exponential(2, n).clip(0.01, 10),
            "packet_rate": rng.normal(200, 100, n).clip(10, 1000),
            "byte_rate": rng.normal(10000, 5000, n).clip(500, 50000),
            "protocol_type": rng.choice([0, 1, 2], n, p=[0.8, 0.1, 0.1]),
            "port_entropy": rng.normal(6.0, 1.0, n).clip(3, 8),
            "payload_entropy": rng.normal(1.5, 0.5, n).clip(0, 4),
            "flag_syn": rng.binomial(1, 0.90, n),
            "flag_ack": rng.binomial(1, 0.05, n),
            "flag_rst": rng.binomial(1, 0.60, n),
            "flow_iat_mean": rng.normal(10, 5, n).clip(0.1, 50),
            "flow_iat_std": rng.normal(5, 2, n).clip(0, 20),
            "label": np.ones(n, dtype=int),
        }

    def _exfil(n: int) -> dict:
        return {
            "packet_size": rng.normal(1400, 100, n).clip(1000, 1500),
            "flow_duration": rng.normal(120, 60, n).clip(10, 600),
            "packet_rate": rng.normal(80, 30, n).clip(5, 300),
            "byte_rate": rng.normal(100000, 40000, n).clip(10000, 500000),
            "protocol_type": rng.choice([0, 1, 2], n, p=[0.3, 0.5, 0.2]),
            "port_entropy": rng.normal(1.0, 0.4, n).clip(0, 3),
            "payload_entropy": rng.normal(7.5, 0.3, n).clip(6, 8),
            "flag_syn": rng.binomial(1, 0.10, n),
            "flag_ack": rng.binomial(1, 0.90, n),
            "flag_rst": rng.binomial(1, 0.01, n),
            "flow_iat_mean": rng.normal(150, 50, n).clip(10, 500),
            "flow_iat_std": rng.normal(60, 20, n).clip(0, 200),
            "label": np.ones(n, dtype=int),
        }

    parts = [_normal(n_normal), _ddos(n_ddos), _portscan(n_portscan), _exfil(n_exfil)]
    df = pd.DataFrame({k: np.concatenate([p[k] for p in parts]) for k in parts[0]})
    return df.sample(frac=1, random_state=seed).reset_index(drop=True)


# ---------------------------------------------------------------------------
# Training
# ---------------------------------------------------------------------------
def train() -> None:
    print("=" * 60)
    print("Threat Detector Training (XGBoost)")
    print("=" * 60)

    # Generate data
    print(f"\nGenerating {N_SAMPLES} synthetic samples ...")
    df = generate_synthetic_data(N_SAMPLES)

    X = df[FEATURE_NAMES].values.astype(np.float32)
    y = df["label"].values

    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=RANDOM_SEED, stratify=y,
    )

    print(f"  Train: {X_train.shape[0]}  |  Test: {X_test.shape[0]}")
    print(f"  Positive rate (train): {y_train.mean():.2%}")

    # Train
    model = XGBClassifier(
        max_depth=6,
        n_estimators=100,
        learning_rate=0.1,
        objective="binary:logistic",
        eval_metric="logloss",
        use_label_encoder=False,
        random_state=RANDOM_SEED,
    )
    model.fit(X_train, y_train, eval_set=[(X_test, y_test)], verbose=False)

    # Evaluate
    y_pred = model.predict(X_test)
    acc = accuracy_score(y_test, y_pred)
    prec = precision_score(y_test, y_pred)
    rec = recall_score(y_test, y_pred)
    f1 = f1_score(y_test, y_pred)

    print(f"\n--- Evaluation ---")
    print(f"  Accuracy:  {acc:.4f}")
    print(f"  Precision: {prec:.4f}")
    print(f"  Recall:    {rec:.4f}")
    print(f"  F1 Score:  {f1:.4f}")

    # Export to ONNX
    print(f"\nExporting to ONNX -> {MODEL_OUTPUT_PATH}")
    export_xgboost_to_onnx(model, FEATURE_NAMES, MODEL_OUTPUT_PATH)

    # Validate
    print("\nValidating exported model ...")
    valid = validate_onnx(MODEL_OUTPUT_PATH)
    if valid:
        print("Model validation PASSED.")
    else:
        print("Model validation FAILED.")

    print("=" * 60)


if __name__ == "__main__":
    train()

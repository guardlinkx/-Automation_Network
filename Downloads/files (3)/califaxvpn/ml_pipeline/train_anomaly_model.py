"""
Train an Isolation Forest anomaly scorer for VPN traffic monitoring.

Generates synthetic normal and anomalous traffic samples, trains an
IsolationForest model, evaluates it, and exports to ONNX format.
"""

import os
import numpy as np
import pandas as pd
from sklearn.ensemble import IsolationForest
from sklearn.metrics import classification_report

from export_onnx import export_sklearn_to_onnx, validate_onnx

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
FEATURE_NAMES = [
    "bytes_per_second",
    "packets_per_second",
    "unique_destinations",
    "avg_packet_size",
    "connection_duration",
    "reconnect_frequency",
    "dns_query_rate",
    "failed_connections",
]

N_NORMAL = 5_000
N_ANOMALOUS = 500
RANDOM_SEED = 42
MODEL_OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "models")
MODEL_OUTPUT_PATH = os.path.join(MODEL_OUTPUT_DIR, "anomaly_scorer.onnx")


# ---------------------------------------------------------------------------
# Synthetic data generation
# ---------------------------------------------------------------------------
def generate_synthetic_data(
    n_normal: int = N_NORMAL,
    n_anomalous: int = N_ANOMALOUS,
    seed: int = RANDOM_SEED,
) -> tuple[pd.DataFrame, np.ndarray]:
    """
    Generate synthetic network-traffic samples.

    Normal traffic follows moderate, well-behaved distributions. Anomalous
    traffic exhibits unusual bursts, high failure rates, or abnormal
    destination counts.

    Returns:
        (DataFrame of features, array of labels where 1=normal, -1=anomaly)
    """
    rng = np.random.RandomState(seed)

    # --- Normal traffic ---
    normal = {
        "bytes_per_second": rng.normal(50_000, 15_000, n_normal).clip(1_000, 150_000),
        "packets_per_second": rng.normal(80, 25, n_normal).clip(5, 300),
        "unique_destinations": rng.poisson(5, n_normal).clip(1, 30),
        "avg_packet_size": rng.normal(600, 150, n_normal).clip(64, 1500),
        "connection_duration": rng.exponential(60, n_normal).clip(1, 600),
        "reconnect_frequency": rng.poisson(1, n_normal).clip(0, 10),
        "dns_query_rate": rng.normal(5, 2, n_normal).clip(0, 20),
        "failed_connections": rng.poisson(0.5, n_normal).clip(0, 5),
    }

    # --- Anomalous traffic ---
    anomalous = {
        "bytes_per_second": rng.normal(300_000, 100_000, n_anomalous).clip(100_000, 1_000_000),
        "packets_per_second": rng.normal(1_000, 400, n_anomalous).clip(200, 5_000),
        "unique_destinations": rng.poisson(50, n_anomalous).clip(20, 200),
        "avg_packet_size": np.where(
            rng.binomial(1, 0.5, n_anomalous),
            rng.normal(80, 20, n_anomalous).clip(40, 150),
            rng.normal(1450, 30, n_anomalous).clip(1400, 1500),
        ),
        "connection_duration": rng.exponential(5, n_anomalous).clip(0.1, 30),
        "reconnect_frequency": rng.poisson(15, n_anomalous).clip(5, 60),
        "dns_query_rate": rng.normal(50, 20, n_anomalous).clip(20, 200),
        "failed_connections": rng.poisson(10, n_anomalous).clip(3, 50),
    }

    df_normal = pd.DataFrame(normal)
    df_anomalous = pd.DataFrame(anomalous)

    df = pd.concat([df_normal, df_anomalous], ignore_index=True)
    labels = np.array([1] * n_normal + [-1] * n_anomalous)

    # Shuffle
    idx = rng.permutation(len(df))
    df = df.iloc[idx].reset_index(drop=True)
    labels = labels[idx]

    return df, labels


# ---------------------------------------------------------------------------
# Training
# ---------------------------------------------------------------------------
def train() -> None:
    print("=" * 60)
    print("Anomaly Scorer Training (Isolation Forest)")
    print("=" * 60)

    # Generate data
    print(f"\nGenerating {N_NORMAL} normal + {N_ANOMALOUS} anomalous samples ...")
    df, labels = generate_synthetic_data()

    X = df[FEATURE_NAMES].values.astype(np.float32)

    print(f"  Total samples: {len(X)}")
    print(f"  Anomaly rate:  {(labels == -1).mean():.2%}")

    # Train (IsolationForest is unsupervised; we use labels only for evaluation)
    model = IsolationForest(
        contamination=0.1,
        n_estimators=100,
        random_state=RANDOM_SEED,
    )
    model.fit(X)

    # Evaluate
    predictions = model.predict(X)
    print(f"\n--- Evaluation (on training data) ---")
    print(classification_report(
        labels, predictions, target_names=["Anomaly (-1)", "Normal (1)"],
    ))

    # Export to ONNX
    print(f"Exporting to ONNX -> {MODEL_OUTPUT_PATH}")
    export_sklearn_to_onnx(model, FEATURE_NAMES, MODEL_OUTPUT_PATH)

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

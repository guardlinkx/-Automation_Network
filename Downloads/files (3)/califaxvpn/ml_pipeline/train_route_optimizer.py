"""
Train a LightGBM regressor for VPN route latency prediction.

Generates synthetic routing data with realistic latency targets, trains a
LightGBM model, evaluates it, and exports to ONNX format.
"""

import os
import numpy as np
import pandas as pd
from sklearn.model_selection import train_test_split
from sklearn.metrics import mean_absolute_error, mean_squared_error, r2_score
from lightgbm import LGBMRegressor

from export_onnx import export_lightgbm_to_onnx, validate_onnx

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
FEATURE_NAMES = [
    "source_region",
    "dest_region",
    "current_load",
    "bandwidth_mbps",
    "hop_count",
    "time_of_day",
    "packet_loss_rate",
    "jitter_ms",
]

# Region encoding: 0=US-East, 1=US-West, 2=EU-West, 3=EU-Central,
#                   4=Asia-East, 5=Asia-South, 6=South-America, 7=Oceania
NUM_REGIONS = 8

N_SAMPLES = 8_000
RANDOM_SEED = 42
MODEL_OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "models")
MODEL_OUTPUT_PATH = os.path.join(MODEL_OUTPUT_DIR, "route_optimizer.onnx")

# Base latency matrix (ms) between regions (symmetric, diagonal = intra-region)
_BASE_LATENCY = np.array([
    [ 10,  60, 80, 90, 180, 200, 120, 220],
    [ 60,  10, 140, 150, 120, 160, 150, 140],
    [ 80, 140, 10,  20, 160, 170, 130, 250],
    [ 90, 150, 20,  10, 150, 160, 140, 240],
    [180, 120, 160, 150, 10,  40, 200, 100],
    [200, 160, 170, 160, 40,  10, 210, 120],
    [120, 150, 130, 140, 200, 210, 10,  230],
    [220, 140, 250, 240, 100, 120, 230,  10],
], dtype=np.float32)


# ---------------------------------------------------------------------------
# Synthetic data generation
# ---------------------------------------------------------------------------
def generate_synthetic_data(
    n_samples: int = N_SAMPLES,
    seed: int = RANDOM_SEED,
) -> pd.DataFrame:
    """
    Generate synthetic routing samples with realistic latency targets.

    Latency is derived from base inter-region latency, adjusted by current
    server load, bandwidth constraints, hop count, time-of-day congestion,
    packet loss, and jitter.
    """
    rng = np.random.RandomState(seed)

    src = rng.randint(0, NUM_REGIONS, n_samples)
    dst = rng.randint(0, NUM_REGIONS, n_samples)
    current_load = rng.uniform(0.0, 1.0, n_samples).astype(np.float32)
    bandwidth = rng.uniform(10.0, 1000.0, n_samples).astype(np.float32)
    hops = rng.randint(1, 15, n_samples).astype(np.float32)
    tod = rng.uniform(0.0, 24.0, n_samples).astype(np.float32)
    pkt_loss = rng.uniform(0.0, 0.10, n_samples).astype(np.float32)
    jitter = rng.exponential(3.0, n_samples).clip(0, 50).astype(np.float32)

    # Compute target latency
    base = np.array([_BASE_LATENCY[s, d] for s, d in zip(src, dst)])
    load_factor = 1.0 + 0.8 * current_load
    bw_factor = 1.0 + 200.0 / bandwidth
    hop_factor = 1.0 + 0.05 * hops
    # Peak-hour congestion (15:00-21:00 UTC)
    peak_mask = ((tod >= 15) & (tod <= 21)).astype(np.float32)
    tod_factor = 1.0 + 0.3 * peak_mask
    loss_factor = 1.0 + 5.0 * pkt_loss

    latency = (
        base * load_factor * bw_factor * hop_factor * tod_factor * loss_factor
        + jitter
        + rng.normal(0, 3, n_samples)  # noise
    ).clip(1.0, 2000.0).astype(np.float32)

    df = pd.DataFrame({
        "source_region": src.astype(np.float32),
        "dest_region": dst.astype(np.float32),
        "current_load": current_load,
        "bandwidth_mbps": bandwidth,
        "hop_count": hops,
        "time_of_day": tod,
        "packet_loss_rate": pkt_loss,
        "jitter_ms": jitter,
        "latency_ms": latency,
    })
    return df


# ---------------------------------------------------------------------------
# Training
# ---------------------------------------------------------------------------
def train() -> None:
    print("=" * 60)
    print("Route Optimizer Training (LightGBM Regressor)")
    print("=" * 60)

    # Generate data
    print(f"\nGenerating {N_SAMPLES} synthetic routing samples ...")
    df = generate_synthetic_data()

    X = df[FEATURE_NAMES].values.astype(np.float32)
    y = df["latency_ms"].values

    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=RANDOM_SEED,
    )

    print(f"  Train: {X_train.shape[0]}  |  Test: {X_test.shape[0]}")
    print(f"  Latency range: [{y.min():.1f}, {y.max():.1f}] ms")
    print(f"  Latency mean:  {y.mean():.1f} ms")

    # Train
    model = LGBMRegressor(
        n_estimators=100,
        max_depth=8,
        learning_rate=0.1,
        num_leaves=63,
        random_state=RANDOM_SEED,
        verbose=-1,
    )
    model.fit(
        X_train, y_train,
        eval_set=[(X_test, y_test)],
        eval_metric="mae",
    )

    # Evaluate
    y_pred = model.predict(X_test)
    mae = mean_absolute_error(y_test, y_pred)
    rmse = np.sqrt(mean_squared_error(y_test, y_pred))
    r2 = r2_score(y_test, y_pred)

    print(f"\n--- Evaluation ---")
    print(f"  MAE:  {mae:.2f} ms")
    print(f"  RMSE: {rmse:.2f} ms")
    print(f"  R2:   {r2:.4f}")

    # Export to ONNX
    print(f"\nExporting to ONNX -> {MODEL_OUTPUT_PATH}")
    export_lightgbm_to_onnx(model, FEATURE_NAMES, MODEL_OUTPUT_PATH)

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

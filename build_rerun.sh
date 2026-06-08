#!/bin/bash
set -e  # stop if any command fails

echo "🔨 Building rerun-cli (Rust)..."
pixi run cargo build --package rerun-cli --release

echo "📂 Copying rerun binary into Python SDK..."
mkdir -p rerun_py/rerun_sdk/rerun_cli/
cp target/release/rerun rerun_py/rerun_sdk/rerun_cli/rerun

echo "🛠️ Building Python wheel..."
pixi run maturin build --manifest-path rerun_py/Cargo.toml --release

echo "🔍 Finding correct wheel..."
# WHEEL=$(ls target/wheels/*linux_x86_64.whl | head -n 1)
WHEEL=$(ls target/wheels/*manylinux*_x86_64.whl | head -n 1)
if [ -z "$WHEEL" ]; then
  echo "❌ ERROR: No linux_x86_64 wheel found!"
  exit 1
fi

echo "📦 Installing $WHEEL into current venv..."
pip install "$WHEEL" --force-reinstall

echo "✅ Done! Ready to import rerun in Python!"

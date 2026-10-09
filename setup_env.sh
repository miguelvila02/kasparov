#!/usr/bin/env bash
# Creates the "kasparov" conda environment with everything needed for
# Phase 0/1: Rust + wasm32 target, wasm-pack, Node.js, wasm-opt.
# Usage:  bash setup_env.sh
set -euo pipefail

ENV_NAME="kasparov"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Make "conda activate" work inside a script
eval "$(conda shell.bash hook)"

# 1. Create (or recreate) the environment
if conda env list | awk '{print $1}' | grep -qx "$ENV_NAME"; then
  echo ">> Environment '$ENV_NAME' already exists, removing it..."
  conda env remove -n "$ENV_NAME" -y
fi
echo ">> Creating environment '$ENV_NAME'..."
conda env create -f "$SCRIPT_DIR/environment.yml"
conda activate "$ENV_NAME"

# 2. Keep Cargo's cache and installed tools inside the environment
conda env config vars set CARGO_HOME="$CONDA_PREFIX/cargo" >/dev/null
conda deactivate
conda activate "$ENV_NAME"

# 3. wasm-pack (prebuilt binary via npm, installed into the env's prefix)
echo ">> Installing wasm-pack..."
npm install -g wasm-pack
# Fallback if the npm install fails (compiles from source, takes a few minutes):
#   cargo install wasm-pack --root "$CONDA_PREFIX"

# 4. Verify
echo ">> Versions:"
rustc --version
cargo --version
node --version
npm --version
wasm-pack --version
wasm-opt --version

echo ">> Checking the wasm32 target compiles..."
TMP="$(mktemp -d)"
echo 'pub fn f() -> i32 { 42 }' > "$TMP/t.rs"
rustc --target wasm32-unknown-unknown --crate-type cdylib -O -o "$TMP/t.wasm" "$TMP/t.rs"
rm -rf "$TMP"
echo ">> wasm32 OK"

echo ""
echo "Done. Activate it with:  conda activate $ENV_NAME"

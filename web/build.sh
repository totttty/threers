#!/usr/bin/env bash
set -euo pipefail

# Build the threers wasm package and emit JS glue into web/pkg.
# Requires: rustup target add wasm32-unknown-unknown && cargo install wasm-bindgen-cli wasm-pack

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WEB="$(cd "$(dirname "$0")" && pwd)"

cd "$ROOT"

PROFILE="${PROFILE:-release}"
PROFILE_FLAG=""
TARGET_DIR="target/wasm32-unknown-unknown/debug"
if [ "$PROFILE" = "release" ]; then
    PROFILE_FLAG="--release"
    TARGET_DIR="target/wasm32-unknown-unknown/release"
fi

echo "==> Building threers ($PROFILE) for wasm32-unknown-unknown..."
cargo build --target wasm32-unknown-unknown --lib $PROFILE_FLAG

echo "==> Generating JS bindings into web/pkg..."
wasm-bindgen \
    "$TARGET_DIR/threers.wasm" \
    --target web \
    --out-dir web/pkg \
    --no-typescript

echo "==> Patching WebGPU device limits for browser compatibility..."
"$WEB/post-build.sh"

echo "==> Done. Open web/index.html with a static server (e.g. python3 -m http.server -d web)."
echo "    Browser must support WebGPU (Chrome 113+ or Firefox Nightly with flag)."

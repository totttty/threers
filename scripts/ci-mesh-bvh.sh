#!/usr/bin/env bash
# CI: mesh-bvh feature flag build + parity scenes.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
echo "==> mesh-bvh CI: build"
MESH_BVH=1 PROFILE=release web/build.sh
echo "==> mesh-bvh CI: feature check"
node web/scripts/check-mesh-bvh.mjs
echo "==> mesh-bvh CI: unit tests"
cargo test --features mesh-bvh mesh_bvh --quiet
echo "==> mesh-bvh CI: parity scenes"
node tests/parity/run-mesh-bvh.js
echo "==> mesh-bvh CI OK"

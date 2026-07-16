#!/usr/bin/env bash
# CI: bvh-csg feature flag build + parity scenes.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
echo "==> bvh-csg CI: build"
BVH_CSG=1 PROFILE=release web/build.sh
echo "==> bvh-csg CI: feature check"
node web/scripts/check-bvh-csg.mjs
echo "==> bvh-csg CI: unit tests"
cargo test --features bvh-csg mesh_bvh --quiet
echo "==> bvh-csg CI: mesh-bvh parity (regression)"
node tests/parity/run-mesh-bvh.js
echo "==> bvh-csg CI: parity scenes"
node tests/parity/run-bvh-csg.js
echo "==> bvh-csg CI OK"

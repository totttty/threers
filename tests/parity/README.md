# Parity tests

Pixel-level regression tests comparing **three.js r165** reference renders to **threers wasm**.

See the [main README](../../README.md#parity-compare-ui) for setup. Quick reference:

```bash
npm install
node server.js              # compare UI at http://localhost:8087
node run.js                 # core suite → out/compare-results.json
node run-mesh-bvh.js        # requires MESH_BVH=1 web/build.sh
node run-bvh-csg.js         # requires BVH_CSG=1 web/build.sh
node generate-scenes.js     # regenerate scene HTML from manifest
node generate-rust-scenes.js
```

Scene pairs live in `scenes/threejs-*.html` and `scenes/threers-*.html`. The compare UI loads threers scenes through `threers-runner.html` (persistent wasm) instead of reloading each iframe.

**CSG topology (Rust):** `cargo test --features bvh-csg --lib csg::step_tests` — exact ordered TriKey for hierarchy steps vs `scenes/rust/*.geom.bin`.

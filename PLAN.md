# threers plan

Roadmap and deferred work for the three.js-compatible Rust/wgpu stack (`web/threejs-shim.js` + wasm core).

## Now

- Core parity suite (91 scenes + generated edge cases)
- Post-processing pass parity (halftone + film perfect; glitch/dotscreen/fxaa approximate — see notes below)

**Approximate postfx (SwiftShader parity run)**

| Scene | Diff | Cause |
|-------|------|--------|
| fxaa | ~2.3% | Composer RT path is 0% vs copy; gap is FXAA edge blending. WebGL FXAA applies subtle changes that differ from WGSL f16-linear sampling on half-float RT. |
| dotscreen | ~4% | Composer copy 0%; gap is `average*10 + pattern` amplifying half-float / sin drift. GLSL pattern pre-bake (WebGL2) wired but same threshold on SwiftShader. |
| glitch | ~10.6% | Snow-free parity scene; production snow uses GLSL `sin()` ≠ WGSL. |
- Offline loader deps (draco, meshopt) under `web/deps/`

## Later

### Ecosystem extensions (feature-flagged)

Optional compatibility with popular three.js add-ons, compiled only when enabled so default wasm stays lean.

| Flag | Package | Purpose |
|------|---------|---------|
| `mesh-bvh` | [three-mesh-bvh](https://github.com/gkjohnson/three-mesh-bvh) | BVH-accelerated raycasting, shapecast, GPU picking helpers |
| `bvh-csg` | [three-bvh-csg](https://github.com/gkjohnson/three-bvh-csg) | Boolean CSG on `BufferGeometry` (depends on `mesh-bvh`) |

**Approach**

- Add Cargo features in `Cargo.toml` (`mesh-bvh`, `bvh-csg`; `bvh-csg` implies `mesh-bvh`).
- Mirror flags in the JS shim build so `threejs-shim.js` can export matching APIs (`MeshBVH`, `StaticGeometryGenerator`, CSG evaluators) without pulling them into the default bundle.
- Implement Rust-side BVH build/query first; wire shim adapters that match three-mesh-bvh / three-bvh-csg call shapes and data layouts.
- Add parity scenes behind the same flags (raycast-heavy picking, simple CSG union/subtract/intersect) — not part of the default 113-scene run.
- Track API variations across package versions (generator options, `MeshBVH` serialization, CSG attribute handling) and gate breaking differences per flag sub-version if needed.

**Non-goals for v1 of these flags**

- No requirement to ship the full three-mesh-bvh shader/debug visualization suite on day one.
- No default enablement; apps opt in explicitly at build time.

### mesh-bvh (implemented)

Build wasm with BVH support:

```bash
MESH_BVH=1 web/build.sh
```

This enables the Cargo `mesh-bvh` feature **and** flips the JS feature flag (`web/features.js` → `features.meshBvh: true`), wiring `mesh-bvh-addon.js` to the real implementation instead of the stub.

Import the addon in browser apps:

```js
import { isMeshBvhEnabled, installMeshBvh, MeshBVH, StaticGeometryGenerator } from './mesh-bvh-addon.js';
if (!isMeshBvhEnabled()) throw new Error('Rebuild with MESH_BVH=1 web/build.sh');
installMeshBvh(THREE);
geom.boundsTree = new MeshBVH(geom);
```

Verify: `node web/scripts/check-mesh-bvh.mjs` (after `MESH_BVH=1` build).

Flagged parity scenes: `tests/parity/scenes-manifest-mesh-bvh.json` — run via `node tests/parity/run-mesh-bvh.js` (after `MESH_BVH=1 web/build.sh`).

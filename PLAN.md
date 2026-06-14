# threers plan

Roadmap and deferred work for the three.js-compatible Rust/wgpu stack (`web/threejs-shim.js` + wasm core).

## Now

- Core parity suite (113 scenes + generated edge cases)
- Post-processing pass parity (halftone + film perfect; glitch/dotscreen/fxaa approximate — see notes below)
- **mesh-bvh v1** — opt-in Cargo/JS feature; 2 flagged parity scenes at 0% pixel diff (see below)
- **mesh-bvh v2** — build strategies, refit, serialize, extended queries, StaticGeometryGenerator; 5 parity scenes at 0% pixel diff; `scripts/ci-mesh-bvh.sh`
- Parity compare UI — orbit controls on all interactive scenes, **Sync orbit** on by default, mesh-bvh scenes in sidebar

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

| Flag | Package | Purpose | Status |
|------|---------|---------|--------|
| `mesh-bvh` | [three-mesh-bvh](https://github.com/gkjohnson/three-mesh-bvh) | BVH-accelerated raycasting, shapecast, GPU picking helpers | **v2 shipped** (core + extended queries) |
| `bvh-csg` | [three-bvh-csg](https://github.com/gkjohnson/three-bvh-csg) | Boolean CSG on `BufferGeometry` (depends on `mesh-bvh`) | Not started |

**Approach**

- Add Cargo features in `Cargo.toml` (`mesh-bvh`, `bvh-csg`; `bvh-csg` implies `mesh-bvh`).
- Mirror flags in the JS shim build so `threejs-shim.js` can export matching APIs (`MeshBVH`, `StaticGeometryGenerator`, CSG evaluators) without pulling them into the default bundle.
- Implement Rust-side BVH build/query first; wire shim adapters that match three-mesh-bvh / three-bvh-csg call shapes and data layouts.
- Add parity scenes behind the same flags (raycast-heavy picking, simple CSG union/subtract/intersect) — not part of the default 113-scene run.
- Track API variations across package versions (generator options, `MeshBVH` serialization, CSG attribute handling) and gate breaking differences per flag sub-version if needed.

**Non-goals for v1 of these flags**

- No requirement to ship the full three-mesh-bvh shader/debug visualization suite on day one.
- No default enablement; apps opt in explicitly at build time.

---

### mesh-bvh — v2 (shipped)

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

**Implemented (Rust + wasm + `web/mesh-bvh-impl.js`)**

| API | Notes |
|-----|-------|
| `MeshBVH` constructor | `CENTER` / `AVERAGE` / `SAH`; `offset` / `count` range build |
| `raycast` / `raycastFirst` | Local-space; sorted hits; backface culling (matches three-mesh-bvh) |
| `shapecast` | `NOT_INTERSECTED` / `INTERSECTED` / `CONTAINED`; JS traversal over wasm `nodeBuffer` |
| `getBoundingBox` | Root AABB |
| `refit` | Rebuild leaf bounds after position-only edits |
| `serialize` / `deserialize` | Versioned binary round-trip |
| `intersectsBox` / `intersectsSphere` | BVH overlap queries |
| `closestPointToPoint` | Closest point on mesh surface |
| `computeBoundsTree` / `disposeBoundsTree` | On `BufferGeometry`; invalidates on geometry edits |
| `acceleratedRaycast` | Patches `Mesh.raycast`; respects `firstHitOnly`; uses `matrixWorld` |
| `StaticGeometryGenerator` | World-space bake + `mergeGeometries`; built-in primitives via `toBufferGeometry()` |
| `GenerateMeshBVHWorker` | Async API stub (main-thread yield; no real worker yet) |
| `installMeshBvh(THREE)` | Prototype patches + `Raycaster.intersectObject` |
| `Raycaster` fast path | `bounds_tree` on `BufferGeometry` when feature enabled |

**Parity**

- Manifest: `tests/parity/scenes-manifest-mesh-bvh.json` (5 scenes)
- Runner: `node tests/parity/run-mesh-bvh.js` (requires `MESH_BVH=1 web/build.sh`)
- CI: `scripts/ci-mesh-bvh.sh` (build + `check-mesh-bvh.mjs` + unit tests + parity)
- Results: `tests/parity/out/compare-results-mesh-bvh.json`
- Compare UI: scenes merged into sidebar; `threers-runner` + `threejs-runner` inject orbit controls; mesh-bvh addon loaded when `features.meshBvh`

| Scene | Visual diff | Meta |
|-------|-------------|------|
| `mesh-bvh-raycast` | 0% | `hitCount` + `hitDistance` match |
| `mesh-bvh-shapecast` | 0% | `shapecastHits` match (225) |
| `mesh-bvh-multihit` | 0% | multi-object sorted hits |
| `mesh-bvh-static-gen` | 0% | `StaticGeometryGenerator` merge + raycast |
| `mesh-bvh-intersects` | 0% | `intersectsBox` / `intersectsSphere` |

**Dev footguns**

- Plain `web/build.sh` sets `features.meshBvh: false` — mesh-bvh scenes fail in compare UI until rebuild with `MESH_BVH=1`.
- mesh-bvh parity is **not** in main `node tests/parity/run.js`; run `run-mesh-bvh.js` or `scripts/ci-mesh-bvh.sh`.

---

### mesh-bvh — v3 (deferred, full three-mesh-bvh parity)

Remaining backlog for 100% three-mesh-bvh coverage:

1. **Workers** — real `GenerateMeshBVHWorker` / `ParallelMeshBVHWorker` (Web Worker wasm build)
2. **Extended queries** — `raycastObject3D`, `intersectsGeometry`, `closestPointToGeometry`, `bvhcast`
3. **Other BVH types** — lines, points, skinned, batched / instanced (`ObjectBVH`, batched bounds trees)
4. **GPU / debug** — `BVHShaderGLSL`, WebGPU/TSL compute raycast, `VertexAttributeTexture`, `MeshBVHHelper`
5. **Parity** — `CONTAINED` shapecast edge case; merge mesh-bvh into main `run.js` CI (optional)
6. **Downstream** — `bvh-csg` feature (implies `mesh-bvh`)

---

### bvh-csg (not started)

Depends on stable `mesh-bvh` v2. See [three-bvh-csg](https://github.com/gkjohnson/three-bvh-csg). Add flagged parity scenes (union / subtract / intersect) when implemented.

# Light probe grids

`LightProbeGrid` brings the position-dependent diffuse global illumination used by the current Three.js light-probe examples to threers. Each probe renders a linear RGBA16F cube map, projects it to nine L2 spherical-harmonic coefficients, and uploads the grid to the renderer. Shaded surfaces trilinearly interpolate nearby probes and evaluate cosine-convolved irradiance in WGSL.

## Run the examples

Build the wasm package and start the existing parity server from the repository root:

```sh
./web/build.sh
node tests/parity/server.js
```

Then open:

- `http://localhost:8087/web/examples/lightprobes-cornell.html`
- `http://localhost:8087/web/examples/lightprobes-sponza.html`

Sponza is loaded from the Khronos glTF Sample Assets repository, so that example needs internet access on first load.

## API

```js
import * as THREE from '../threejs-shim.js';
import { LightProbeGrid, LightProbeGridHelper } from '../light-probe-grid.js';

const grid = new LightProbeGrid(5.6, 4.7, 5.6, new THREE.Vector3(4, 4, 4));
grid.position.set(0, 2.45, 0);
scene.add(grid);

await grid.bake(renderer, scene, {
    cubemapSize: 8,
    near: 0.05,
    far: 20,
    bounces: 1,
    onProgress: ({ ratio }) => console.log(`${Math.round(ratio * 100)}%`),
});

const helper = new LightProbeGridHelper(grid, 0.08);
scene.add(helper);
```

## Reusing baked probes

Probe baking renders six cube faces per probe and performs a GPU readback for
each cube. Use `bakeCached()` to pay that cost only when the scene or bake
configuration changes:

```js
await grid.bakeCached(renderer, scene, {
    cacheKey: 'my-scene-probes-v3:r6:cube32:b0',
    cacheUrl: './assets/my-scene-probes.lpb', // optional bundled cache
    cubemapSize: 32,
    near: 0.05,
    far: 20,
    bounces: 0,
});
```

The lookup order is a bundled `.lpb` asset, then IndexedDB, then a live bake.
After a fallback bake the new cache is written to IndexedDB. Set `forceBake:
true` to ignore existing data while authoring lighting.

`cacheKey` is the invalidation contract. Change it whenever scene geometry,
materials, lights, probe placement, bake settings, or relevant renderer shader
behavior changes. Corrupt, truncated, mismatched, non-finite, oversized, or
wrong-version files are rejected by Rust and fall back to baking.

For native tooling, `BakedLightProbeGrid::new()`, `to_bytes()`, and
`from_bytes()` expose the same format directly. The file stores exact `f32` L2
SH coefficients plus resolution, bounds, settings, fingerprint, and a whole-file
CRC. It intentionally does not store the adapter-dependent half-float GPU atlas.

The included examples ship caches for their default settings:

- Cornell `6³`: 31,192 bytes instead of 1,296 cube-face renders.
- Sponza `10×7×7`, one bounce: 70,648 bytes instead of 5,880 cube-face renders.

- Set `grid.visible` to enable or disable its indirect lighting.
- Call `grid.setResolution(new THREE.Vector3(x, y, z))` before rebaking to change density.
- `bounces: 0` captures direct lighting once; each additional bounce captures the prior probe pass.
- The renderer accepts at most 2,048 probes. Every resolution axis must be at least two.
- `LightProbeGridHelper` uses per-instance colors derived from each probe's constant SH coefficient. Hide it during presentation or baking if desired.

The reusable CPU projection routines live in `web/light-probe-math.js`; `npm run test:light-probes` covers half-float decoding, constant-radiance projection, position-dependent interpolation, and interleaved glTF accessors.

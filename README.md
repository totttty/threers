# threers

A **three.js–inspired 3D library for Rust**, backed by [wgpu](https://github.com/gfx-rs/wgpu). The same Rust core runs natively (winit) and in the browser (WebAssembly), with a JavaScript shim that maps the API onto familiar `THREE.*` symbols.

## Features

- Scene graph (`Object3D`, `Scene`, transforms, layers)
- Buffer geometries, PBR materials, lights, shadows, fog
- wgpu renderer with post-processing (FXAA, bloom, SSAO, glitch, halftone, …)
- Loaders (glTF, OBJ, STL, HDR, …), animation, controls, helpers
- **Web**: `web/threejs-shim.js` + wasm pkg — drop-in parity with three.js r165 patterns
- **Native**: winit examples for desktop development and debugging

## Quick start

### Native (desktop)

Requires Rust 1.75+, a working wgpu backend (Metal/Vulkan/DX12), and dev dependencies from `Cargo.toml`.

```bash
cargo run --example cube          # spinning PBR cube
cargo run --example scene_graph   # hierarchy + lights
cargo run --example controls_orbit
```

### Web (browser)

Build the wasm package and serve the repo root (or use the parity server below):

```bash
wasm-pack build --target web --out-dir web/pkg
bash web/post-build.sh          # Safari glue + TypeScript defs
```

Then load `web/examples/index.html` or any page that imports `/web/threejs-shim.js` and calls `initThreers('/web/pkg/threers_bg.wasm')`.

```javascript
import THREE, { initThreers } from '/web/threejs-shim.js';

await initThreers('/web/pkg/threers_bg.wasm');
const renderer = await THREE.WebGLRenderer.create(document.querySelector('canvas'));
// … same patterns as three.js
```

## Parity compare UI

Side-by-side **three.js r165 vs threers wasm** for 113 regression scenes:

```bash
cd tests/parity
npm install                    # puppeteer, pixelmatch (first time)
node server.js                 # http://localhost:8087
```

Open **http://localhost:8087/** — navigate scenes, toggle light/dark theme, view source (JS / TS / Rust tabs).

Run the full pixel-diff suite:

```bash
cd tests/parity
node run.js                    # writes tests/parity/out/compare-results.json
```

After changing Rust or wasm, rebuild and hard-refresh the browser (⌘⇧R) or click **↻** in the compare UI so cached wasm/shim are busted.

## Project layout

```
src/                 Rust library (scene graph, renderer, loaders, …)
  renderer/          wgpu pipelines, post-fx WGSL, GPU mesh/texture caches
  wasm.rs            #[wasm_bindgen] exports (web only)
web/
  threejs-shim.js    THREE-compatible JS API over wasm
  pkg/               wasm-pack output (threers.js, threers_bg.wasm)
  post-build.sh      Patches wasm glue for Safari; generates .d.ts
tests/parity/        Scene pairs, compare UI, pixel-diff runner
examples/            Native winit demos
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Browser: three.js app code (or parity scene HTML)      │
│       ↓ import                                            │
│  web/threejs-shim.js  ──►  wasm WebRenderer / WebScene  │
└──────────────────────────────┬──────────────────────────┘
                               ↓
┌──────────────────────────────────────────────────────────┐
│  Rust: Scene → Renderer (wgpu) → surface or RenderTarget │
│        Materials / lights / post-fx (EffectComposer)     │
└──────────────────────────────────────────────────────────┘
```

- **Native path**: `Renderer::new(device, queue, format)` → `render()` / `render_to()`.
- **Web path**: `WebRenderer` owns the wgpu surface for a `<canvas>`; render targets and post-fx are registered by integer id across the JS boundary.
- **Post-fx**: A single WGSL module (`src/renderer/shader.rs`) switches on `effect_kind` (copy, FXAA, glitch, bloom, …). The JS `EffectComposer` mirrors three.js pass order.

## Development

| Task | Command |
|------|---------|
| Native check | `cargo build` |
| Wasm release | `wasm-pack build --target web --out-dir web/pkg && bash web/post-build.sh` |
| Parity scenes | `cd tests/parity && node generate-scenes.js` |
| Rust scene snippets (parity UI Rust tab) | `cd tests/parity && node generate-rust-scenes.js` |
| Regenerate shim types | `node web/scripts/generate-shim-types.mjs` (also runs in post-build) |

## Status

Active development toward **pixel parity** with three.js r165 on the parity scene set. Some scenes are approximate (PMREM, SSR, etc.) — the compare UI marks these and stores diff percentages in `compare-results.json`.

## License

MIT

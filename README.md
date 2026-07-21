# threers

A **drop-in three.js replacement** for Rust and the browser, backed by [wgpu](https://github.com/gfx-rs/wgpu). The same Rust core runs natively (winit) and as WebAssembly, with a JavaScript shim that exposes the familiar `THREE.*` API so existing three.js code can run with minimal changes.

**Current version:** [0.0.2](CHANGELOG.md) · [Changelog](CHANGELOG.md)

## Table of contents

- [Features](#features)
- [Quick start](#quick-start)
  - [Native (desktop)](#native-desktop)
  - [Optional: mesh-bvh / CSG](#optional-mesh-bvh--csg)
  - [Headless render & video export](#headless-render--video-export)
  - [Native codecs (GIF, APNG, VP9, HEVC)](#native-codecs-gif-apng-vp9-hevc)
  - [Web (browser)](#web-browser)
- [Parity compare UI](#parity-compare-ui)
- [Project layout](#project-layout)
- [Architecture](#architecture)
- [Development](#development)
- [Status](#status)
- [License](#license)

## Features

- Scene graph (`Object3D`, `Scene`, transforms, layers)
- Buffer geometries, PBR materials, lights, shadows, fog
- Custom WGSL materials via `ShaderMaterial`
- wgpu renderer with post-processing (FXAA, bloom, SSAO, glitch, halftone, …)
- Loaders (glTF, OBJ, STL, HDR, …), animation, controls, helpers
- **Headless** offscreen render → tightly packed RGBA (`HeadlessRenderer`, native)
- **Video export** (`video`) — frame sequence → ffmpeg; native GIF when `native-codec` is on
- **Native codecs** (`native-codec`) — pure-Rust GIF (encode/decode), APNG, VP9/WebM, HEVC/MP4; wasm-safe
- **Opt-in mesh BVH** (`mesh-bvh`) — accelerated raycast / shapecast (three-mesh-bvh–compatible)
- **Opt-in CSG** (`bvh-csg`) — boolean ops on `BufferGeometry` (three-bvh-csg–compatible); Rust native + JS addon
- **Web**: `web/threejs-shim.js` + wasm — drop-in `THREE.*` replacement targeting three.js r165
- **Native**: winit examples for desktop development and debugging

## Quick start

### Native (desktop)

Requires Rust 1.75+, a working wgpu backend (Metal/Vulkan/DX12), and dev dependencies from `Cargo.toml`.

```bash
cargo run --example cube          # spinning PBR cube
cargo run --example scene_graph   # hierarchy + lights
cargo run --example controls_orbit
cargo run --example shader_material
```

### Optional: mesh-bvh / CSG

```bash
# BVH picking demo
cargo run --example mesh_bvh_picking --features mesh-bvh

# Hierarchy CSG (exact TriKey parity with JS three-bvh-csg@0.0.16)
cargo run --example bvh_csg_hierarchy --features bvh-csg
cargo run --example bvh_csg_steps --features bvh-csg -- 2 --live
```

```rust
// Cargo.toml: threers = { version = "0.0.2", features = ["bvh-csg"] }
use threers::{CsgBrush, CsgEvaluator, BoxGeometry, SUBTRACTION};

let mut ev = CsgEvaluator::new();
let mut a = CsgBrush::new(BoxGeometry::new(2.0, 2.0, 2.0));
let mut b = CsgBrush::new(BoxGeometry::new(1.0, 1.0, 1.0));
let _geom = ev.evaluate(&mut a, &mut b, SUBTRACTION);
```

### Headless render & video export

Native-only. `HeadlessRenderer` owns the wgpu device and an offscreen target; `export_video` pipes RGBA frames to ffmpeg (or the in-process GIF encoder when both features are enabled).

```bash
cargo run --release --example headless_video --features video
```

```rust
// Cargo.toml: threers = { version = "0.0.2", features = ["video"] }
use threers::{export_video, HeadlessRenderer, VideoCodec, VideoOptions};

let mut hr = HeadlessRenderer::builder().size(1280, 720).build().unwrap();
let opts = VideoOptions::new("out.mp4").fps(30).codec(VideoCodec::H264);
// export_video(w, h, frame_count, &opts, |i| { /* advance scene */; hr.render_to_rgba(...) })
```

GIF without spawning ffmpeg:

```rust
// features = ["video", "native-codec"]
let opts = VideoOptions::new("out.gif")
    .fps(12)
    .codec(VideoCodec::Gif)
    .transparent(true)
    .gif_colors(128);
```

### Native codecs (GIF, APNG, VP9, HEVC)

Enable `native-codec` for dependency-free encoders (and a GIF decoder) that also build on `wasm32`:

```rust
// Cargo.toml: threers = { version = "0.0.2", features = ["native-codec"] }
use threers::{encode_gif, decode_gif, GifEncoder, GifOptions, PaletteMode};

let mut enc = GifEncoder::new(64, 64, 0)
    .colors(64)
    .dither(true)
    .diff_rects(true)
    .palette_mode(PaletteMode::Local);
// enc.add_frame(&rgba, 1, 10);
let bytes = enc.finish();
let (_info, frames) = decode_gif(&bytes).unwrap();
```

See `threers::codec` module docs and `tests/{gif,apng,webm,hevc_*}.rs` for VP9/WebM, APNG, and HEVC usage.

### Web (browser)

Build the wasm package and serve the repo root (or use the parity server below):

```bash
# Default (no mesh-bvh / CSG addon)
wasm-pack build --target web --out-dir web/pkg && bash web/post-build.sh

# Or via the feature build scripts:
MESH_BVH=1 web/build.sh          # mesh-bvh addon
BVH_CSG=1 web/build.sh           # CSG addon (implies mesh-bvh)
```

Then load `web/examples/index.html` or any page that imports `/web/threejs-shim.js` and calls `initThreers('/web/pkg/threers_bg.wasm')`.

```javascript
import THREE, { initThreers } from '/web/threejs-shim.js';

await initThreers('/web/pkg/threers_bg.wasm');
const renderer = await THREE.WebGLRenderer.create(document.querySelector('canvas'));
// … same patterns as three.js
```

CSG in the browser (after `BVH_CSG=1` build):

```javascript
import { installBvhCsg, Brush, Evaluator, ADDITION } from '/web/bvh-csg-addon.js';
installBvhCsg(THREE);
```

## Parity compare UI

Side-by-side **three.js r165 vs threers wasm** for 113 regression scenes:

```bash
cd tests/parity
npm install                    # puppeteer, pixelmatch (first time)
node server.js                 # http://localhost:8087
```

Open **http://localhost:8087/** — navigate scenes, toggle light/dark theme, view source (JS / TS / Rust tabs).

```bash
cd tests/parity
node run.js                    # core suite → out/compare-results.json
node run-mesh-bvh.js           # mesh-bvh scenes (MESH_BVH=1 build)
node run-bvh-csg.js            # CSG scenes (BVH_CSG=1 build)
```

CI helpers: `scripts/ci-mesh-bvh.sh`, `scripts/ci-bvh-csg.sh`.

After changing Rust or wasm, rebuild and hard-refresh the browser (⌘⇧R) or click **↻** in the compare UI so cached wasm/shim are busted.

## Project layout

```
src/                 Rust library (scene graph, renderer, loaders, …)
  codec/             Pure-Rust media codecs (`native-codec`)
  video.rs           Frame-sequence export (`video`, native)
  renderer/          wgpu pipelines, post-fx, headless offscreen
  materials/         PBR + ShaderMaterial
  csg/               Rust CSG (`bvh-csg`) — Evaluator, Brush, hierarchy
  mesh_bvh/          Rust BVH (`mesh-bvh`)
  wasm.rs            #[wasm_bindgen] exports (web only)
web/
  threejs-shim.js    THREE-compatible JS API over wasm
  csg/               JS three-bvh-csg port (loaded when BVH_CSG=1)
  mesh-bvh-*.js      mesh-bvh addon / stub
  pkg/               wasm-pack output (threers.js, threers_bg.wasm)
  post-build.sh      Patches wasm glue for Safari; generates .d.ts
tests/               Codec / video integration tests + parity/
examples/            Native demos (cube, headless_video, shader_material, …)
scripts/             Feature CI (mesh-bvh, bvh-csg)
CHANGELOG.md         Release history
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Browser: three.js app code (or parity scene HTML)      │
│       ↓ import                                            │
│  web/threejs-shim.js  ──►  wasm WebRenderer / WebScene  │
│  (+ optional mesh-bvh / bvh-csg addons)                 │
└──────────────────────────────┬──────────────────────────┘
                               ↓
┌──────────────────────────────────────────────────────────┐
│  Rust: Scene → Renderer (wgpu) → surface or RenderTarget │
│        Materials / lights / post-fx (EffectComposer)     │
│        Optional: MeshBvh, CsgEvaluator                   │
│        Optional: HeadlessRenderer → video / native-codec │
└──────────────────────────────────────────────────────────┘
```

- **Native path**: `Renderer::new(device, queue, format)` → `render()` / `render_to()`; or `HeadlessRenderer` for offscreen RGBA.
- **Web path**: `WebRenderer` owns the wgpu surface for a `<canvas>`; render targets and post-fx are registered by integer id across the JS boundary.
- **Post-fx**: A single WGSL module (`src/renderer/shader.rs`) switches on `effect_kind` (copy, FXAA, glitch, bloom, …). The JS `EffectComposer` mirrors three.js pass order.
- **CSG**: Rust `CsgEvaluator` for native/tests; browser parity scenes use the JS port under `web/csg/` with wasm BVH for splits.
- **Codecs**: `native-codec` encodes/decodes in-process (no ffmpeg); `video` shells out to ffmpeg except for GIF when both features are enabled.

## Development

| Task | Command |
|------|---------|
| Native check | `cargo build` |
| With CSG | `cargo build --features bvh-csg` |
| Video + codecs | `cargo build --features "video,native-codec"` |
| CSG unit/parity tests | `cargo test --features bvh-csg --lib csg` |
| GIF codec tests | `cargo test --features native-codec --lib codec::gif --test gif` |
| Wasm release | `wasm-pack build --target web --out-dir web/pkg && bash web/post-build.sh` |
| Feature wasm | `BVH_CSG=1 web/build.sh` / `MESH_BVH=1 web/build.sh` |
| Parity scenes | `cd tests/parity && node generate-scenes.js` |
| Rust scene snippets (parity UI Rust tab) | `cd tests/parity && node generate-rust-scenes.js` |
| Regenerate shim types | `node web/scripts/generate-shim-types.mjs` (also runs in post-build) |
| Docs | `cargo doc --features "video,native-codec" --open` |

## Status

Active development toward **pixel parity** with three.js r165 on the core parity scene set. Opt-in **mesh-bvh** and **bvh-csg** suites report 0% pixel diff on their dedicated manifests; hierarchy CSG also has **exact ordered TriKey** topology parity in Rust vs JS.

Some core scenes are approximate (PMREM, SSR, etc.) — the compare UI marks these and stores diff percentages in `compare-results.json`. See [PLAN.md](PLAN.md) for feature roadmap notes and [CHANGELOG.md](CHANGELOG.md) for release history.

## License

MIT

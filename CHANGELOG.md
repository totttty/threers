# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.2] — 2026-07-21

### Added
- `native-codec` feature: pure-Rust, wasm-safe media codecs (no ffmpeg / C bindings).
  - HEVC / H.265 encoder and MP4 mux helpers.
  - VP9 encoder (intra + inter ladder) and WebM mux with alpha (`BlockAdditional`).
  - APNG encoder from RGBA frames.
  - Full GIF89a encoder: local/global/auto palettes, octree & median-cut quantization, Floyd–Steinberg dithering, dirty-rect differencing, transparency, disposal modes, lossy indexing, deferred/adaptive LZW clears, comments, interlacing, and incremental `GifWriter`.
  - Native GIF decoder (`GifDecoder` / `decode_gif`) with disposal compositing, interlace, and structured errors.
- `video` feature: `export_video` frame-sequence export via system ffmpeg; with `native-codec`, `VideoCodec::Gif` streams through `GifWriter`.
- Headless RGBA rendering path and related examples/tests for codec round-trips.

### Changed
- Public re-exports for codec and video APIs when the corresponding features are enabled.
- Web package versions (`web/package.json`, `web/pkg`) aligned to the crate version.
- README updated with table of contents, headless/video/codec quick starts, and feature overview.
- Expanded rustdoc on GIF encode/decode, video export, headless rendering, and `ShaderMaterial`.
- Per-format `export_*` examples, `encode_animation_rgba` / `BrowserCodec`, and browser export demo (`NATIVE_CODEC=1`).
- JS/TS video export API: `encodeVideoFrames`, `exportSceneVideo`, `BrowserVideoFormat` (`web/video-export.js` + shim).

## [0.0.1] — 2026-07-16

### Added
- Initial public crate: three.js–shaped wgpu renderer for native and wasm.
- Scene graph, geometries, PBR materials, lights, post-processing, loaders, controls, helpers.
- Opt-in `mesh-bvh` and `bvh-csg` (three-mesh-bvh / three-bvh-csg parity).
- Web shim (`THREE.*`) and parity tooling.

[0.0.2]: https://github.com/eugenehp/threers/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/eugenehp/threers/releases/tag/v0.0.1

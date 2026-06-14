//! Post-processing pipeline. Mirrors three.js's `EffectComposer` /
//! `RenderPass` / `ShaderPass`. Each `Pass` (other than `RenderPass`) carries
//! its own WGSL fragment shader; the composer assembles them into a
//! ping-pong fullscreen chain on top of the offscreen `RenderTarget`.

mod composer;
mod passes;

pub use composer::EffectComposer;
pub use passes::{
    Pass, RenderPass, BloomPass, FxaaPass, OutlinePass, ToneMappingPass,
    FilmPass, GlitchPass, SsaoPass, SsrPass, CopyPass,
};

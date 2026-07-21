use crate::math::Color;

/// Marker trait for a post-process pass.
pub trait Pass: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &'static str;
    /// True for the scene-rendering pass (always first).
    fn is_render(&self) -> bool {
        false
    }
    /// WGSL fragment shader source for fullscreen passes that operate on the
    /// previous color buffer (`prev_tex` + `prev_sampler` at @group(0)).
    /// Render passes (`is_render = true`) return None.
    fn shader(&self) -> Option<&'static str> {
        None
    }
}

/// Renders the scene into the composer's read target. Always the first pass.
#[derive(Debug, Default)]
pub struct RenderPass;
impl Pass for RenderPass {
    fn name(&self) -> &'static str {
        "render"
    }
    fn is_render(&self) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct BloomPass {
    pub strength: f32,
    pub radius: f32,
    pub threshold: f32,
}
impl Default for BloomPass {
    fn default() -> Self {
        Self {
            strength: 1.0,
            radius: 0.4,
            threshold: 0.85,
        }
    }
}
impl Pass for BloomPass {
    fn name(&self) -> &'static str {
        "bloom"
    }
    fn shader(&self) -> Option<&'static str> {
        Some(BLOOM_FRAG)
    }
}

#[derive(Debug, Default)]
pub struct FxaaPass;
impl Pass for FxaaPass {
    fn name(&self) -> &'static str {
        "fxaa"
    }
    fn shader(&self) -> Option<&'static str> {
        Some(FXAA_FRAG)
    }
}

#[derive(Debug)]
pub struct OutlinePass {
    pub edge_strength: f32,
    pub edge_glow: f32,
    pub edge_thickness: f32,
    pub edge_color: Color,
}
impl Default for OutlinePass {
    fn default() -> Self {
        Self {
            edge_strength: 3.0,
            edge_glow: 0.0,
            edge_thickness: 1.0,
            edge_color: Color::WHITE,
        }
    }
}
impl Pass for OutlinePass {
    fn name(&self) -> &'static str {
        "outline"
    }
    fn shader(&self) -> Option<&'static str> {
        Some(OUTLINE_FRAG)
    }
}

#[derive(Debug)]
pub struct ToneMappingPass {
    pub exposure: f32,
}
impl Default for ToneMappingPass {
    fn default() -> Self {
        Self { exposure: 1.0 }
    }
}
impl Pass for ToneMappingPass {
    fn name(&self) -> &'static str {
        "tone_mapping"
    }
    fn shader(&self) -> Option<&'static str> {
        Some(TONE_FRAG)
    }
}

/// Film grain + scanlines pass. Mirrors three.js's FilmPass.
#[derive(Debug)]
pub struct FilmPass {
    pub grain_intensity: f32,
    pub scanline_intensity: f32,
    pub scanline_count: f32,
    pub time: f32,
}
impl Default for FilmPass {
    fn default() -> Self {
        Self {
            grain_intensity: 0.5,
            scanline_intensity: 0.05,
            scanline_count: 480.0,
            time: 0.0,
        }
    }
}
impl Pass for FilmPass {
    fn name(&self) -> &'static str {
        "film"
    }
    fn shader(&self) -> Option<&'static str> {
        Some(FILM_FRAG)
    }
}

#[derive(Debug, Default)]
pub struct GlitchPass;
impl Pass for GlitchPass {
    fn name(&self) -> &'static str {
        "glitch"
    }
    fn shader(&self) -> Option<&'static str> {
        Some(GLITCH_FRAG)
    }
}

#[derive(Debug, Default)]
pub struct SsaoPass;
impl Pass for SsaoPass {
    fn name(&self) -> &'static str {
        "ssao"
    }
    // Implemented in POSTFX_SHADER kind 11 (JS SSAOPass drives the GPU path).
    fn shader(&self) -> Option<&'static str> {
        Some(COPY_FRAG)
    }
}

#[derive(Debug, Default)]
pub struct SsrPass;
impl Pass for SsrPass {
    fn name(&self) -> &'static str {
        "ssr"
    }
    // Implemented in POSTFX_SHADER kind 12 (JS SSRPass drives the GPU path).
    fn shader(&self) -> Option<&'static str> {
        Some(COPY_FRAG)
    }
}

#[derive(Debug, Default)]
pub struct CopyPass;
impl Pass for CopyPass {
    fn name(&self) -> &'static str {
        "copy"
    }
    fn shader(&self) -> Option<&'static str> {
        Some(COPY_FRAG)
    }
}

// ---- WGSL fragment shaders ----
// All passes share a common vertex shader (full-screen triangle) provided by
// EffectComposer.

pub(crate) const COPY_FRAG: &str = r#"
@group(0) @binding(0) var prev_tex : texture_2d<f32>;
@group(0) @binding(1) var prev_sampler : sampler;
@fragment
fn fs_main(@location(0) uv : vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(prev_tex, prev_sampler, uv);
}
"#;

pub(crate) const FXAA_FRAG: &str = r#"
@group(0) @binding(0) var prev_tex : texture_2d<f32>;
@group(0) @binding(1) var prev_sampler : sampler;
fn rgb_luma(c : vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.299, 0.587, 0.114)); }
@fragment
fn fs_main(@location(0) uv : vec2<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(prev_tex));
    let inv = vec2<f32>(1.0) / dims;
    let c   = textureSample(prev_tex, prev_sampler, uv);
    let nw  = textureSample(prev_tex, prev_sampler, uv + vec2<f32>(-inv.x, -inv.y)).rgb;
    let ne  = textureSample(prev_tex, prev_sampler, uv + vec2<f32>( inv.x, -inv.y)).rgb;
    let sw  = textureSample(prev_tex, prev_sampler, uv + vec2<f32>(-inv.x,  inv.y)).rgb;
    let se  = textureSample(prev_tex, prev_sampler, uv + vec2<f32>( inv.x,  inv.y)).rgb;
    let l_c  = rgb_luma(c.rgb);
    let l_nw = rgb_luma(nw);
    let l_ne = rgb_luma(ne);
    let l_sw = rgb_luma(sw);
    let l_se = rgb_luma(se);
    let l_min = min(l_c, min(min(l_nw, l_ne), min(l_sw, l_se)));
    let l_max = max(l_c, max(max(l_nw, l_ne), max(l_sw, l_se)));
    if (l_max - l_min < 0.05) {
        return c;
    }
    var dir = vec2<f32>(
        -((l_nw + l_ne) - (l_sw + l_se)),
         ((l_nw + l_sw) - (l_ne + l_se)),
    );
    let rcp_dir = 1.0 / (min(abs(dir.x), abs(dir.y)) + 0.00078125);
    dir = clamp(dir * rcp_dir, vec2<f32>(-8.0), vec2<f32>(8.0)) * inv;
    let s1 = textureSample(prev_tex, prev_sampler, uv + dir * (1.0 / 3.0 - 0.5)).rgb;
    let s2 = textureSample(prev_tex, prev_sampler, uv + dir * (2.0 / 3.0 - 0.5)).rgb;
    let blend = (s1 + s2) * 0.5;
    return vec4<f32>(blend, c.a);
}
"#;

pub(crate) const BLOOM_FRAG: &str = r#"
@group(0) @binding(0) var prev_tex : texture_2d<f32>;
@group(0) @binding(1) var prev_sampler : sampler;
fn luma(c : vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.299, 0.587, 0.114)); }
// Jimenez interleaved-gradient noise → a per-pixel angle in [0, 1).
fn ign(frag : vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(frag, vec2<f32>(0.06711056, 0.00583715))));
}
@fragment
fn fs_main(@location(0) uv : vec2<f32>) -> @location(0) vec4<f32> {
    let c = textureSample(prev_tex, prev_sampler, uv);
    let dims = vec2<f32>(textureDimensions(prev_tex));
    let px = vec2<f32>(1.0) / dims;
    let min_dim = min(dims.x, dims.y);
    // Halo radius as a fraction of the image → resolution-independent, wide.
    let radius = min_dim * 0.02;
    let threshold = 0.55;

    // A regular grid of wide-spaced taps replicates a thin bright arc into a
    // lattice of concentric ghost copies (the "ripples"). Sample a golden-angle
    // spiral instead — no aligned grid, no rings — and rotate the whole spiral
    // by a per-pixel angle so any residual structure dithers into fine noise
    // that reads as a smooth glow rather than banded rings.
    let TAPS = 128;
    let GOLDEN = 2.3999632;                 // golden angle (radians)
    let base = ign(uv * dims) * 6.2831853;  // per-pixel spiral rotation
    var sum = vec3<f32>(0.0);
    var wsum = 1e-6;
    for (var i: i32 = 0; i < TAPS; i = i + 1) {
        let fi = f32(i) + 0.5;
        let rn = sqrt(fi / f32(TAPS));       // even areal density across the disk
        let ang = fi * GOLDEN + base;
        let off = vec2<f32>(cos(ang), sin(ang)) * (rn * radius) * px;
        let s = textureSampleLevel(prev_tex, prev_sampler, uv + off, 0.0).rgb;
        // Keep the full color of bright pixels (soft threshold), not just the
        // sliver above the threshold — otherwise the bloom is near-invisible.
        let mask = smoothstep(threshold, threshold + 0.25, luma(s));
        let w = exp(-rn * rn * 3.0);         // Gaussian falloff toward the rim
        sum = sum + s * mask * w;
        wsum = wsum + w;
    }
    let bloom = sum / wsum;
    return vec4<f32>(c.rgb + bloom * 0.9, c.a);
}
"#;

pub(crate) const OUTLINE_FRAG: &str = r#"
@group(0) @binding(0) var prev_tex : texture_2d<f32>;
@group(0) @binding(1) var prev_sampler : sampler;
fn luma(c : vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.299, 0.587, 0.114)); }
@fragment
fn fs_main(@location(0) uv : vec2<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(prev_tex));
    let inv  = vec2<f32>(1.0) / dims;
    let l    = luma(textureSample(prev_tex, prev_sampler, uv).rgb);
    let lx0  = luma(textureSample(prev_tex, prev_sampler, uv + vec2<f32>(-inv.x, 0.0)).rgb);
    let lx1  = luma(textureSample(prev_tex, prev_sampler, uv + vec2<f32>( inv.x, 0.0)).rgb);
    let ly0  = luma(textureSample(prev_tex, prev_sampler, uv + vec2<f32>(0.0, -inv.y)).rgb);
    let ly1  = luma(textureSample(prev_tex, prev_sampler, uv + vec2<f32>(0.0,  inv.y)).rgb);
    let edge = abs(lx0 - lx1) + abs(ly0 - ly1);
    let c    = textureSample(prev_tex, prev_sampler, uv).rgb;
    return vec4<f32>(mix(c, vec3<f32>(1.0), clamp(edge * 3.0, 0.0, 1.0)), 1.0);
}
"#;

pub(crate) const TONE_FRAG: &str = r#"
@group(0) @binding(0) var prev_tex : texture_2d<f32>;
@group(0) @binding(1) var prev_sampler : sampler;
fn aces(x : vec3<f32>) -> vec3<f32> {
    let a = 2.51; let b = 0.03; let c = 2.43; let d = 0.59; let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}
@fragment
fn fs_main(@location(0) uv : vec2<f32>) -> @location(0) vec4<f32> {
    let c = textureSample(prev_tex, prev_sampler, uv);
    return vec4<f32>(aces(c.rgb), c.a);
}
"#;

pub(crate) const FILM_FRAG: &str = r#"
@group(0) @binding(0) var prev_tex : texture_2d<f32>;
@group(0) @binding(1) var prev_sampler : sampler;
fn rand(co : vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}
@fragment
fn fs_main(@location(0) uv : vec2<f32>) -> @location(0) vec4<f32> {
    let c = textureSample(prev_tex, prev_sampler, uv);
    let scan = 0.95 + 0.05 * sin(uv.y * 480.0 * 3.14159);
    let grain = (rand(uv * 1024.0) - 0.5) * 0.06;
    return vec4<f32>(c.rgb * scan + vec3<f32>(grain), c.a);
}
"#;

pub(crate) const GLITCH_FRAG: &str = r#"
@group(0) @binding(0) var prev_tex : texture_2d<f32>;
@group(0) @binding(1) var prev_sampler : sampler;
@fragment
fn fs_main(@location(0) uv : vec2<f32>) -> @location(0) vec4<f32> {
    let band = step(0.97, fract(uv.y * 21.7));
    let shifted = textureSample(prev_tex, prev_sampler, uv + vec2<f32>(band * 0.04, 0.0));
    return shifted;
}
"#;

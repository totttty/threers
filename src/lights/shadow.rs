/// Shadow-map configuration carried by lights that can cast shadows.
/// Mirrors three.js's `LightShadow` for the parts the renderer reads.
/// The depth pre-pass + sampling are scaffolded but not yet driving pixels;
/// see `renderer/renderer.rs` for the next-step implementation hooks.
#[derive(Debug, Clone, Copy)]
pub struct ShadowSettings {
    pub map_size: u32,
    pub bias: f32,
    pub normal_bias: f32,
    /// Orthographic frustum size (in world units) around the light target.
    pub camera_size: f32,
    pub camera_near: f32,
    pub camera_far: f32,
}

impl Default for ShadowSettings {
    fn default() -> Self {
        Self {
            map_size: 1024,
            bias: 0.0,
            normal_bias: 0.0,
            camera_size: 5.0,
            camera_near: 0.1,
            camera_far: 500.0,
        }
    }
}

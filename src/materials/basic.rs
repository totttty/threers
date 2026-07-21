use crate::math::Color;
use crate::textures::Texture;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BasicMaterial {
    pub color: Color,
    pub wireframe: bool,
    pub map: Option<Arc<Texture>>,
    pub transparent: bool,
    pub opacity: f32,
    /// 0=FrontSide (default cull-back), 1=BackSide (cull-front),
    /// 2=DoubleSide (no cull). Both 1 and 2 currently route to the
    /// no-cull pipeline.
    pub side: u32,
    /// When true, only shadow darkness is visible (three.js ShadowMaterial).
    pub shadow_only: bool,
    pub alpha_test: f32,
}

impl Default for BasicMaterial {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            wireframe: false,
            map: None,
            transparent: false,
            opacity: 1.0,
            side: 0,
            shadow_only: false,
            alpha_test: 0.0,
        }
    }
}

impl BasicMaterial {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            wireframe: false,
            map: None,
            transparent: false,
            opacity: 1.0,
            side: 0,
            shadow_only: false,
            alpha_test: 0.0,
        }
    }

    pub fn wireframe(mut self, w: bool) -> Self {
        self.wireframe = w;
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }
}

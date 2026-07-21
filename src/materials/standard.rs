use crate::math::{Color, Vector2};
use crate::textures::Texture;
use std::sync::Arc;

/// PBR roughness/metalness workflow. Matches three.js's `MeshStandardMaterial`.
#[derive(Debug, Clone)]
pub struct StandardMaterial {
    pub color: Color,
    pub emissive: Color,
    pub emissive_intensity: f32,
    pub roughness: f32,
    pub metalness: f32,
    pub ao_intensity: f32,
    pub normal_scale: Vector2,
    pub opacity: f32,
    pub wireframe: bool,

    // Textures (all optional). When set, the corresponding GPU texture slot is
    // bound and the WGSL flag enables sampling.
    pub map: Option<Arc<Texture>>,
    pub normal_map: Option<Arc<Texture>>,
    pub roughness_map: Option<Arc<Texture>>,
    pub metalness_map: Option<Arc<Texture>>,
    pub ao_map: Option<Arc<Texture>>,
    pub emissive_map: Option<Arc<Texture>>,

    pub side: u32,
}

impl Default for StandardMaterial {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            emissive: Color::BLACK,
            emissive_intensity: 1.0,
            roughness: 1.0,
            metalness: 0.0,
            ao_intensity: 1.0,
            normal_scale: Vector2::new(1.0, 1.0),
            opacity: 1.0,
            wireframe: false,
            map: None,
            normal_map: None,
            roughness_map: None,
            metalness_map: None,
            ao_map: None,
            emissive_map: None,
            side: 0,
        }
    }
}

impl StandardMaterial {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            ..Default::default()
        }
    }

    pub fn with_roughness(mut self, r: f32) -> Self {
        self.roughness = r;
        self
    }
    pub fn with_metalness(mut self, m: f32) -> Self {
        self.metalness = m;
        self
    }
    pub fn with_emissive(mut self, c: Color, intensity: f32) -> Self {
        self.emissive = c;
        self.emissive_intensity = intensity;
        self
    }
    pub fn with_map(mut self, t: Arc<Texture>) -> Self {
        self.map = Some(t);
        self
    }
    pub fn with_normal_map(mut self, t: Arc<Texture>) -> Self {
        self.normal_map = Some(t);
        self
    }
    pub fn with_roughness_map(mut self, t: Arc<Texture>) -> Self {
        self.roughness_map = Some(t);
        self
    }
    pub fn with_metalness_map(mut self, t: Arc<Texture>) -> Self {
        self.metalness_map = Some(t);
        self
    }
    pub fn with_ao_map(mut self, t: Arc<Texture>) -> Self {
        self.ao_map = Some(t);
        self
    }
    pub fn with_emissive_map(mut self, t: Arc<Texture>) -> Self {
        self.emissive_map = Some(t);
        self
    }
}

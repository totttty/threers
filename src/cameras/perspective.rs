use super::Camera;
use crate::math::{Matrix4, Vector3};

#[derive(Debug, Clone)]
pub struct PerspectiveCamera {
    pub position: Vector3,
    pub target: Vector3,
    pub up: Vector3,
    pub fov: f32, // radians
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    /// When set, overrides the computed perspective matrix (Reflector oblique clip).
    pub projection_override: Option<[f32; 16]>,
}

impl PerspectiveCamera {
    /// `fov_deg` matches three.js's degree-based API.
    pub fn new(fov_deg: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 5.0),
            target: Vector3::ZERO,
            up: Vector3::UP,
            fov: fov_deg.to_radians(),
            aspect,
            near,
            far,
            projection_override: None,
        }
    }

    pub fn look_at(&mut self, target: Vector3) -> &mut Self {
        self.target = target;
        self
    }
}

impl Camera for PerspectiveCamera {
    fn view_matrix(&self) -> Matrix4 {
        Matrix4::look_at(self.position, self.target, self.up)
    }
    fn projection_matrix(&self) -> Matrix4 {
        if let Some(m) = self.projection_override {
            return Matrix4 { elements: m };
        }
        Matrix4::perspective(self.fov, self.aspect, self.near, self.far)
    }
    fn position(&self) -> Vector3 {
        self.position
    }
    fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }
    fn near_far(&self) -> (f32, f32) {
        (self.near, self.far)
    }
}

use crate::math::{Vector3, Matrix4};
use super::Camera;

#[derive(Debug, Clone)]
pub struct OrthographicCamera {
    pub position: Vector3,
    pub target: Vector3,
    pub up: Vector3,
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub near: f32,
    pub far: f32,
}

impl OrthographicCamera {
    pub fn new(left: f32, right: f32, top: f32, bottom: f32, near: f32, far: f32) -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 5.0),
            target: Vector3::ZERO,
            up: Vector3::UP,
            left, right, top, bottom, near, far,
        }
    }
}

impl Camera for OrthographicCamera {
    fn view_matrix(&self) -> Matrix4 {
        Matrix4::look_at(self.position, self.target, self.up)
    }
    fn projection_matrix(&self) -> Matrix4 {
        Matrix4::orthographic(self.left, self.right, self.top, self.bottom, self.near, self.far)
    }
    fn position(&self) -> Vector3 { self.position }
    fn set_aspect(&mut self, aspect: f32) {
        // Preserve vertical span; rescale horizontal symmetrically.
        let v_half = (self.top - self.bottom).abs() * 0.5;
        let h_half = v_half * aspect;
        let cx = (self.left + self.right) * 0.5;
        self.left  = cx - h_half;
        self.right = cx + h_half;
    }
    fn near_far(&self) -> (f32, f32) { (self.near, self.far) }
}

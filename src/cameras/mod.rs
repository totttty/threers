//! Cameras: position/orientation in world space plus a projection matrix.

mod perspective;
mod orthographic;

pub use perspective::PerspectiveCamera;
pub use orthographic::OrthographicCamera;

use crate::math::{Vector3, Matrix4};
use crate::core::Layers;

/// Unified camera trait used by the Renderer.
pub trait Camera {
    fn view_matrix(&self) -> Matrix4;
    fn projection_matrix(&self) -> Matrix4;
    fn position(&self) -> Vector3;
    fn set_aspect(&mut self, aspect: f32);

    /// Clip-space near/far planes (for SSAO depth encoding in normal prepass).
    fn near_far(&self) -> (f32, f32) { (0.1, 1000.0) }

    /// Layer mask. Meshes are visible to this camera iff `camera.layers().test(&mesh.layers)`.
    fn layers(&self) -> Layers { Layers::default() }
}

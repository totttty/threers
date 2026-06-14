use crate::cameras::PerspectiveCamera;
use crate::math::{Quaternion, Vector2, Vector3};
use super::PointerEvent;

/// Sphere-projection rotation: the pointer is projected onto a virtual unit
/// sphere; movement on the sphere rotates the camera around the target.
/// Mirrors three.js's `ArcballControls` (simplified — no pivot/IK).
pub struct ArcballControls {
    pub target: Vector3,
    pub rotate_speed: f32,
    pub zoom_speed: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    last_ndc: Option<Vector2>,
}

impl ArcballControls {
    pub fn new(camera: &PerspectiveCamera) -> Self {
        Self {
            target: camera.target,
            rotate_speed: 1.0,
            zoom_speed: 1.0,
            min_distance: 0.1,
            max_distance: 1000.0,
            last_ndc: None,
        }
    }

    pub fn update(&mut self, ndc: Vector2, ev: PointerEvent, camera: &mut PerspectiveCamera) {
        if ev.rotating {
            if let Some(prev) = self.last_ndc {
                let a = project_to_sphere(prev);
                let b = project_to_sphere(ndc);
                if a.length_sq() > 1e-6 && b.length_sq() > 1e-6 {
                    let axis = a.cross(b);
                    let cos_a = a.dot(b).clamp(-1.0, 1.0);
                    let angle = cos_a.acos() * self.rotate_speed;
                    if axis.length_sq() > 1e-8 {
                        let axis = axis.normalize();
                        let q = Quaternion::from_axis_angle(axis, -angle);
                        let offset = camera.position - self.target;
                        camera.position = self.target + offset.apply_quaternion(q);
                        camera.up = camera.up.apply_quaternion(q);
                    }
                }
            }
            self.last_ndc = Some(ndc);
        } else {
            self.last_ndc = None;
        }
        if ev.wheel != 0.0 {
            let offset = camera.position - self.target;
            let factor = (1.0 - ev.wheel * 0.001 * self.zoom_speed).clamp(0.1, 10.0);
            let new_len = (offset.length() * factor).clamp(self.min_distance, self.max_distance);
            camera.position = self.target + offset.normalize() * new_len;
        }
        camera.target = self.target;
    }
}

fn project_to_sphere(ndc: Vector2) -> Vector3 {
    let d_sq = ndc.x * ndc.x + ndc.y * ndc.y;
    if d_sq <= 1.0 {
        Vector3::new(ndc.x, ndc.y, (1.0 - d_sq).sqrt())
    } else {
        let inv = 1.0 / d_sq.sqrt();
        Vector3::new(ndc.x * inv, ndc.y * inv, 0.0)
    }
}

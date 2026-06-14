use crate::cameras::PerspectiveCamera;
use crate::math::{Quaternion, Vector3};
use super::PointerEvent;

/// Free-rotation orbit (no up-vector lock). Mirrors three.js's `TrackballControls`.
#[derive(Debug, Clone, Copy)]
pub struct TrackballControls {
    pub target: Vector3,
    pub rotate_speed: f32,
    pub zoom_speed: f32,
    pub pan_speed: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

impl TrackballControls {
    pub fn new(camera: &PerspectiveCamera) -> Self {
        Self {
            target: camera.target,
            rotate_speed: 1.0,
            zoom_speed: 1.0,
            pan_speed: 1.0,
            min_distance: 0.1,
            max_distance: 1000.0,
        }
    }

    pub fn update(&mut self, ev: PointerEvent, camera: &mut PerspectiveCamera, viewport: (f32, f32)) {
        let (_, h) = viewport;
        if ev.rotating {
            // Trackball: convert mouse delta to a rotation around the axis
            // perpendicular to the cursor motion in screen space.
            let dx = -ev.dx / h * std::f32::consts::PI * self.rotate_speed;
            let dy = -ev.dy / h * std::f32::consts::PI * self.rotate_speed;
            let offset = camera.position - self.target;
            let forward = (-offset).normalize();
            let right = forward.cross(camera.up).normalize();
            let up = right.cross(forward).normalize();
            // Compose rotations.
            let q_x = Quaternion::from_axis_angle(up, dx);
            let q_y = Quaternion::from_axis_angle(right, dy);
            let q = q_x.multiply(q_y);
            let rotated = offset.apply_quaternion(q);
            camera.position = self.target + rotated;
            camera.up = up.apply_quaternion(q);
        }
        if ev.panning {
            let offset = camera.position - self.target;
            let dist = offset.length();
            let half_fov = camera.fov * 0.5;
            let world_per_pixel = 2.0 * dist * half_fov.tan() / h;
            let forward = (-offset).normalize();
            let right = forward.cross(camera.up).normalize();
            let up = right.cross(forward).normalize();
            let pan = right * (-ev.dx * world_per_pixel * self.pan_speed)
                + up * (ev.dy * world_per_pixel * self.pan_speed);
            self.target = self.target + pan;
            camera.position = camera.position + pan;
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

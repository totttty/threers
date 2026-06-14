use crate::cameras::PerspectiveCamera;
use crate::math::{Spherical, Vector3};
use super::PointerEvent;

/// Spherical orbit camera controls — rotate, pan, zoom around `target`.
/// Mirrors three.js's `OrbitControls`.
#[derive(Debug, Clone, Copy)]
pub struct OrbitControls {
    pub target: Vector3,
    pub min_distance: f32,
    pub max_distance: f32,
    pub min_polar_angle: f32,
    pub max_polar_angle: f32,
    pub rotate_speed: f32,
    pub zoom_speed: f32,
    pub pan_speed: f32,
    pub damping: f32,
    spherical: Spherical,
    pan_offset: Vector3,
}

impl OrbitControls {
    pub fn new(camera: &PerspectiveCamera) -> Self {
        let offset = camera.position - camera.target;
        let spherical = Spherical::from_vector3(offset);
        Self {
            target: camera.target,
            min_distance: 0.1,
            max_distance: 1000.0,
            min_polar_angle: 0.0,
            max_polar_angle: std::f32::consts::PI,
            rotate_speed: 1.0,
            zoom_speed: 1.0,
            pan_speed: 1.0,
            damping: 0.0,
            spherical,
            pan_offset: Vector3::ZERO,
        }
    }

    /// Apply one frame of input. `viewport` is (width, height) in pixels for
    /// pan-distance scaling.
    pub fn update(&mut self, ev: PointerEvent, camera: &mut PerspectiveCamera, viewport: (f32, f32)) {
        let (w, h) = viewport;
        let half_fov = camera.fov * 0.5;
        let dist = self.spherical.radius;

        if ev.rotating {
            // Convert pixel delta to angle delta. Use viewport height as the
            // reference (matches three.js's pixel-to-angle mapping).
            let dtheta = -ev.dx / h * std::f32::consts::PI * 2.0 * self.rotate_speed;
            let dphi   = -ev.dy / h * std::f32::consts::PI * 2.0 * self.rotate_speed;
            self.spherical.theta += dtheta;
            self.spherical.phi = (self.spherical.phi + dphi).clamp(self.min_polar_angle, self.max_polar_angle);
        }
        if ev.panning {
            // Pan in screen space: vertical = projected world distance per pixel.
            let world_per_pixel = 2.0 * dist * half_fov.tan() / h;
            // Recompute camera basis.
            let offset = self.spherical.to_vector3();
            let forward = (-offset).normalize();
            let world_up = Vector3::UP;
            let right = forward.cross(world_up).normalize();
            let up = right.cross(forward).normalize();
            self.pan_offset = self.pan_offset
                + right * (-ev.dx * world_per_pixel * self.pan_speed)
                + up * (ev.dy * world_per_pixel * self.pan_speed);
        }
        if ev.wheel != 0.0 {
            let factor = (1.0 - ev.wheel * 0.001 * self.zoom_speed).clamp(0.1, 10.0);
            self.spherical.radius = (self.spherical.radius * factor).clamp(self.min_distance, self.max_distance);
        }

        self.target = self.target + self.pan_offset;
        self.pan_offset = Vector3::ZERO;
        let offset = self.spherical.to_vector3();
        camera.target = self.target;
        camera.position = self.target + offset;
        // Aspect untouched.
        let _ = w;
    }
}

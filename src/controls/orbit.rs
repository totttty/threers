use super::PointerEvent;
use crate::cameras::PerspectiveCamera;
use crate::math::{Spherical, Vector3};

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

    /// Rebuild spherical state from the camera's current pose (parity orbit sync).
    pub fn reseed_from_camera(&mut self, camera: &PerspectiveCamera) {
        self.target = camera.target;
        self.spherical = Spherical::from_vector3(camera.position - camera.target);
        self.pan_offset = Vector3::ZERO;
    }

    /// Apply one frame of input. `viewport` is (width, height) in pixels for
    /// pan-distance scaling.
    pub fn update(
        &mut self,
        ev: PointerEvent,
        camera: &mut PerspectiveCamera,
        viewport: (f32, f32),
    ) {
        let (w, h) = viewport;
        let half_fov = camera.fov * 0.5;
        let dist = self.spherical.radius;
        let mut changed = ev.rotating || ev.panning || ev.wheel != 0.0;

        if ev.rotating {
            // Convert pixel delta to angle delta. Use viewport height as the
            // reference (matches three.js's pixel-to-angle mapping).
            let dtheta = -ev.dx / h * std::f32::consts::PI * 2.0 * self.rotate_speed;
            let dphi = -ev.dy / h * std::f32::consts::PI * 2.0 * self.rotate_speed;
            self.spherical.theta += dtheta;
            self.spherical.phi =
                (self.spherical.phi + dphi).clamp(self.min_polar_angle, self.max_polar_angle);
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
            changed = true;
        }
        if ev.wheel != 0.0 {
            let factor = (1.0 - ev.wheel * 0.001 * self.zoom_speed).clamp(0.1, 10.0);
            self.spherical.radius =
                (self.spherical.radius * factor).clamp(self.min_distance, self.max_distance);
        }

        // Idle frames must not rewrite the camera — external sync may have set
        // the wasm pose without updating our spherical state yet.
        if !changed {
            return;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controls::PointerEvent;

    #[test]
    fn idle_update_preserves_external_camera_pose() {
        let mut cam = PerspectiveCamera::new(50.0, 1.0, 0.1, 100.0);
        cam.position = Vector3::new(0.0, 0.0, 5.0);
        let mut orbit = OrbitControls::new(&cam);
        // External sync moves the camera without reseeding orbit.
        cam.position = Vector3::new(-3.8, 2.9, 1.25);
        cam.target = Vector3::ZERO;
        let idle = PointerEvent {
            dx: 0.0,
            dy: 0.0,
            wheel: 0.0,
            rotating: false,
            panning: false,
        };
        orbit.update(idle, &mut cam, (800.0, 600.0));
        assert!((cam.position.x + 3.8).abs() < 1e-4);
        assert!((cam.position.y - 2.9).abs() < 1e-4);
        assert!((cam.position.z - 1.25).abs() < 1e-4);
    }

    #[test]
    fn reseed_from_camera_refreshes_spherical() {
        let mut cam = PerspectiveCamera::new(50.0, 1.0, 0.1, 100.0);
        cam.position = Vector3::new(0.0, 0.0, 5.0);
        let mut orbit = OrbitControls::new(&cam);
        cam.position = Vector3::new(-3.8, 2.9, 1.25);
        orbit.reseed_from_camera(&cam);
        let idle = PointerEvent {
            dx: 0.0,
            dy: 0.0,
            wheel: 0.0,
            rotating: false,
            panning: false,
        };
        orbit.update(idle, &mut cam, (800.0, 600.0));
        assert!((cam.position.x + 3.8).abs() < 1e-4);
        let moved = PointerEvent {
            dx: 0.0,
            dy: 0.0,
            wheel: 0.0,
            rotating: false,
            panning: false,
        };
        orbit.update(moved, &mut cam, (800.0, 600.0));
        assert!((cam.position.x + 3.8).abs() < 1e-4);
    }
}

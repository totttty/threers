use super::PointerEvent;
use crate::cameras::PerspectiveCamera;
use crate::math::{Quaternion, Vector3};

/// Yaw+pitch from cursor motion, WASD-style position delta from `move_input`.
/// Mirrors three.js's `FirstPersonControls`.
#[derive(Debug, Clone, Copy)]
pub struct FirstPersonControls {
    pub look_speed: f32,
    pub move_speed: f32,
    pub yaw: f32,
    pub pitch: f32,
    /// Optional WASD axes — (forward, right, up) in -1..=1 each.
    pub move_input: Vector3,
}

impl FirstPersonControls {
    pub fn new(camera: &PerspectiveCamera) -> Self {
        // Initialize yaw/pitch from the existing camera orientation.
        let dir = (camera.target - camera.position).normalize();
        let yaw = dir.x.atan2(dir.z);
        let pitch = dir.y.asin();
        Self {
            look_speed: 1.0,
            move_speed: 1.0,
            yaw,
            pitch,
            move_input: Vector3::ZERO,
        }
    }

    pub fn update(&mut self, ev: PointerEvent, camera: &mut PerspectiveCamera, dt: f32) {
        if ev.rotating {
            self.yaw -= ev.dx * 0.002 * self.look_speed;
            self.pitch -= ev.dy * 0.002 * self.look_speed;
            let half = std::f32::consts::FRAC_PI_2 - 0.01;
            self.pitch = self.pitch.clamp(-half, half);
        }
        let cp = self.pitch.cos();
        let forward =
            Vector3::new(self.yaw.sin() * cp, self.pitch.sin(), self.yaw.cos() * cp).normalize();
        let world_up = Vector3::new(0.0, 1.0, 0.0);
        let right = forward.cross(world_up).normalize();

        let mov =
            forward * self.move_input.x + right * self.move_input.y + world_up * self.move_input.z;
        camera.position = camera.position + mov * (self.move_speed * dt);
        camera.target = camera.position + forward;
        let _ = Quaternion::identity();
    }
}

use super::PointerEvent;
use crate::cameras::PerspectiveCamera;
use crate::math::Vector3;

/// FPS-style controls where the cursor is locked and any cursor motion is a
/// yaw/pitch delta. Caller is responsible for actually locking the pointer
/// (winit's `Window::set_cursor_grab` + `set_cursor_visible(false)` on native;
/// `requestPointerLock` on web). Mirrors three.js's `PointerLockControls`.
pub struct PointerLockControls {
    pub move_input: Vector3,
    pub move_speed: f32,
    pub look_speed: f32,
    yaw: f32,
    pitch: f32,
}

impl PointerLockControls {
    pub fn new(camera: &PerspectiveCamera) -> Self {
        let dir = (camera.target - camera.position).normalize();
        let yaw = dir.x.atan2(dir.z);
        let pitch = dir.y.asin();
        Self {
            move_input: Vector3::ZERO,
            move_speed: 5.0,
            look_speed: 1.0,
            yaw,
            pitch,
        }
    }

    pub fn update(&mut self, ev: PointerEvent, camera: &mut PerspectiveCamera, dt: f32) {
        // Pointer-locked: every event treats dx/dy as look delta (no button needed).
        self.yaw -= ev.dx * 0.002 * self.look_speed;
        self.pitch -= ev.dy * 0.002 * self.look_speed;
        let half = std::f32::consts::FRAC_PI_2 - 0.01;
        self.pitch = self.pitch.clamp(-half, half);
        let cp = self.pitch.cos();
        let forward =
            Vector3::new(self.yaw.sin() * cp, self.pitch.sin(), self.yaw.cos() * cp).normalize();
        let world_up = Vector3::new(0.0, 1.0, 0.0);
        let right = forward.cross(world_up).normalize();
        let mov =
            forward * self.move_input.x + right * self.move_input.y + world_up * self.move_input.z;
        camera.position = camera.position + mov * (self.move_speed * dt);
        camera.target = camera.position + forward;
    }
}

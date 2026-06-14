use super::Quaternion;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Euler {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Euler {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn to_quaternion(self) -> Quaternion {
        Quaternion::from_euler_xyz(self.x, self.y, self.z)
    }
}

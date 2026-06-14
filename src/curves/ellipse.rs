use std::f32::consts::PI;
use crate::math::Vector2;
use super::Curve2;

/// Arc / ellipse / circle (with `x_radius == y_radius`). Matches three.js.
#[derive(Debug, Clone, Copy)]
pub struct EllipseCurve {
    pub center: Vector2,
    pub x_radius: f32,
    pub y_radius: f32,
    pub a_start: f32,
    pub a_end: f32,
    pub clockwise: bool,
    pub rotation: f32,
}

impl EllipseCurve {
    pub const fn new(center: Vector2, x_radius: f32, y_radius: f32, a_start: f32, a_end: f32, clockwise: bool, rotation: f32) -> Self {
        Self { center, x_radius, y_radius, a_start, a_end, clockwise, rotation }
    }
}

impl Curve2 for EllipseCurve {
    fn get_point(&self, t: f32) -> Vector2 {
        let two_pi = PI * 2.0;
        let mut delta_angle = self.a_end - self.a_start;
        let same_points = delta_angle.abs() < f32::EPSILON;
        while delta_angle < 0.0 { delta_angle += two_pi; }
        while delta_angle > two_pi { delta_angle -= two_pi; }
        if delta_angle < f32::EPSILON {
            delta_angle = if same_points { 0.0 } else { two_pi };
        }
        if self.clockwise && !same_points {
            delta_angle = if (delta_angle - two_pi).abs() < f32::EPSILON { -two_pi } else { delta_angle - two_pi };
        }
        let angle = self.a_start + t * delta_angle;
        let x = self.x_radius * angle.cos();
        let y = self.y_radius * angle.sin();
        let (cr, sr) = self.rotation.sin_cos();
        Vector2::new(self.center.x + cr * x - sr * y, self.center.y + sr * x + cr * y)
    }
}

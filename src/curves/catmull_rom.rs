use crate::math::Vector3;
use super::Curve3;

/// Centripetal Catmull-Rom spline through 3D points. Matches three.js's
/// `CatmullRomCurve3` (default tension 0.5, type "centripetal").
#[derive(Debug, Clone)]
pub struct CatmullRomCurve3 {
    pub points: Vec<Vector3>,
    pub closed: bool,
    pub tension: f32,
}

impl CatmullRomCurve3 {
    pub fn new(points: Vec<Vector3>) -> Self {
        Self { points, closed: false, tension: 0.5 }
    }
}

impl Curve3 for CatmullRomCurve3 {
    fn get_point(&self, t: f32) -> Vector3 {
        let n = self.points.len();
        if n == 0 { return Vector3::ZERO; }
        if n == 1 { return self.points[0]; }
        let segments = if self.closed { n } else { n - 1 };
        let p = t * segments as f32;
        let i = (p.floor() as usize).min(segments - 1);
        let weight = p - i as f32;
        let idx = |k: i32| -> Vector3 {
            let m = if self.closed {
                ((i as i32 + k).rem_euclid(n as i32)) as usize
            } else {
                (i as i32 + k).clamp(0, n as i32 - 1) as usize
            };
            self.points[m]
        };
        let p0 = idx(-1);
        let p1 = idx(0);
        let p2 = idx(1);
        let p3 = idx(2);
        catmull_rom(p0, p1, p2, p3, weight, self.tension)
    }
}

fn catmull_rom(p0: Vector3, p1: Vector3, p2: Vector3, p3: Vector3, t: f32, tension: f32) -> Vector3 {
    let t2 = t * t;
    let t3 = t2 * t;
    let m1 = (p2 - p0) * tension;
    let m2 = (p3 - p1) * tension;
    p1 * (2.0 * t3 - 3.0 * t2 + 1.0)
        + p2 * (-2.0 * t3 + 3.0 * t2)
        + m1 * (t3 - 2.0 * t2 + t)
        + m2 * (t3 - t2)
}

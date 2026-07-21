use super::Curve3;
use crate::math::Vector3;

/// Non-Uniform Rational B-Spline curve. Mirrors three.js's `NURBSCurve`.
/// `control_points` are 4D (x, y, z, weight); `knots` are a non-decreasing
/// knot vector of length `control_points.len() + degree + 1`.
#[derive(Debug, Clone)]
pub struct NURBSCurve {
    pub degree: usize,
    pub knots: Vec<f32>,
    pub control_points: Vec<[f32; 4]>,
    pub t_start: f32,
    pub t_end: f32,
}

impl NURBSCurve {
    pub fn new(degree: usize, knots: Vec<f32>, control_points: Vec<[f32; 4]>) -> Self {
        let t_start = knots[degree];
        let t_end = knots[knots.len() - 1 - degree];
        Self {
            degree,
            knots,
            control_points,
            t_start,
            t_end,
        }
    }
}

impl Curve3 for NURBSCurve {
    fn get_point(&self, t: f32) -> Vector3 {
        let u = self.t_start + t * (self.t_end - self.t_start);
        let p = de_boor(self.degree, &self.knots, &self.control_points, u);
        if p[3] != 0.0 {
            Vector3::new(p[0] / p[3], p[1] / p[3], p[2] / p[3])
        } else {
            Vector3::new(p[0], p[1], p[2])
        }
    }
}

/// NURBS surface — bidirectional De Boor evaluation. `cps[i*nu + j]` is the
/// control point at row i / col j, in homogeneous (x, y, z, w) form.
#[derive(Debug, Clone)]
pub struct NURBSSurface {
    pub degree_u: usize,
    pub degree_v: usize,
    pub knots_u: Vec<f32>,
    pub knots_v: Vec<f32>,
    /// `cols × rows` (rows-major). `cols = knots_u.len() - degree_u - 1`,
    /// `rows = knots_v.len() - degree_v - 1`.
    pub control_points: Vec<[f32; 4]>,
    pub cols: usize,
    pub rows: usize,
}

impl NURBSSurface {
    pub fn point(&self, u: f32, v: f32) -> Vector3 {
        // De Boor along u for each row, then along v.
        let mut row_pts: Vec<[f32; 4]> = Vec::with_capacity(self.rows);
        for r in 0..self.rows {
            let row_cps: Vec<[f32; 4]> = (0..self.cols)
                .map(|c| self.control_points[r * self.cols + c])
                .collect();
            row_pts.push(de_boor(self.degree_u, &self.knots_u, &row_cps, u));
        }
        let p = de_boor(self.degree_v, &self.knots_v, &row_pts, v);
        if p[3] != 0.0 {
            Vector3::new(p[0] / p[3], p[1] / p[3], p[2] / p[3])
        } else {
            Vector3::new(p[0], p[1], p[2])
        }
    }
}

/// De Boor's recursive algorithm in homogeneous space.
fn de_boor(degree: usize, knots: &[f32], cps: &[[f32; 4]], u: f32) -> [f32; 4] {
    if cps.is_empty() {
        return [0.0; 4];
    }
    // Find span index k such that knots[k] <= u < knots[k+1].
    let n = cps.len();
    let mut k = degree;
    for i in degree..n {
        if u >= knots[i] && u < knots[i + 1] {
            k = i;
            break;
        }
    }
    if u >= *knots.last().unwrap() {
        k = n - 1;
    }

    // Working set: copy degree+1 control points around span.
    let mut d: Vec<[f32; 4]> = (0..=degree)
        .map(|i| {
            let idx = if k >= degree {
                (k - degree + i).min(n - 1)
            } else {
                i.min(n - 1)
            };
            cps[idx]
        })
        .collect();

    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = k.saturating_sub(degree) + j;
            let denom = knots[i + degree + 1 - r] - knots[i];
            let alpha = if denom > 0.0 {
                (u - knots[i]) / denom
            } else {
                0.0
            };
            let a = d[j - 1];
            let b = d[j];
            d[j] = [
                a[0] * (1.0 - alpha) + b[0] * alpha,
                a[1] * (1.0 - alpha) + b[1] * alpha,
                a[2] * (1.0 - alpha) + b[2] * alpha,
                a[3] * (1.0 - alpha) + b[3] * alpha,
            ];
        }
    }
    d[degree]
}

use super::{Box3, Matrix4, Plane, Sphere, Triangle, Vector3};

/// Mirrors three.js's `Ray`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Vector3,
    pub direction: Vector3,
}

impl Default for Ray {
    fn default() -> Self {
        Self {
            origin: Vector3::ZERO,
            direction: Vector3::new(0.0, 0.0, -1.0),
        }
    }
}

impl Ray {
    pub const fn new(origin: Vector3, direction: Vector3) -> Self {
        Self { origin, direction }
    }

    pub fn at(&self, t: f32) -> Vector3 {
        self.origin + self.direction * t
    }

    pub fn look_at(&mut self, target: Vector3) -> &mut Self {
        self.direction = (target - self.origin).normalize();
        self
    }

    pub fn recast(&self, t: f32) -> Self {
        Self {
            origin: self.at(t),
            direction: self.direction,
        }
    }

    pub fn closest_point_to_point(&self, p: Vector3) -> Vector3 {
        let t = (p - self.origin).dot(self.direction);
        if t < 0.0 {
            self.origin
        } else {
            self.at(t)
        }
    }

    pub fn distance_to_point(&self, p: Vector3) -> f32 {
        self.distance_sq_to_point(p).sqrt()
    }

    pub fn distance_sq_to_point(&self, p: Vector3) -> f32 {
        let t = (p - self.origin).dot(self.direction);
        if t < 0.0 {
            (self.origin - p).length_sq()
        } else {
            (self.at(t) - p).length_sq()
        }
    }

    /// Returns the t along the ray where it enters the sphere, or None.
    pub fn intersect_sphere(&self, s: &Sphere) -> Option<f32> {
        let oc = s.center - self.origin;
        let tca = oc.dot(self.direction);
        let d2 = oc.length_sq() - tca * tca;
        let r2 = s.radius * s.radius;
        if d2 > r2 {
            return None;
        }
        let thc = (r2 - d2).sqrt();
        let t0 = tca - thc;
        let t1 = tca + thc;
        if t1 < 0.0 {
            return None;
        }
        if t0 < 0.0 {
            Some(t1)
        } else {
            Some(t0)
        }
    }

    pub fn intersects_sphere(&self, s: &Sphere) -> bool {
        self.intersect_sphere(s).is_some()
    }

    /// Slab method. Returns entry t along ray or None.
    pub fn intersect_box(&self, b: &Box3) -> Option<f32> {
        let inv = Vector3::new(
            1.0 / self.direction.x,
            1.0 / self.direction.y,
            1.0 / self.direction.z,
        );
        let (mut tmin, mut tmax) = slab(self.origin.x, b.min.x, b.max.x, inv.x);
        let (tymin, tymax) = slab(self.origin.y, b.min.y, b.max.y, inv.y);
        if tmin > tymax || tymin > tmax {
            return None;
        }
        if tymin > tmin {
            tmin = tymin;
        }
        if tymax < tmax {
            tmax = tymax;
        }
        let (tzmin, tzmax) = slab(self.origin.z, b.min.z, b.max.z, inv.z);
        if tmin > tzmax || tzmin > tmax {
            return None;
        }
        if tzmin > tmin {
            tmin = tzmin;
        }
        if tzmax < tmax {
            tmax = tzmax;
        }
        if tmax < 0.0 {
            return None;
        }
        Some(if tmin >= 0.0 { tmin } else { tmax })
    }

    pub fn intersects_box(&self, b: &Box3) -> bool {
        self.intersect_box(b).is_some()
    }

    pub fn intersect_plane(&self, plane: &Plane) -> Option<f32> {
        let denom = plane.normal.dot(self.direction);
        if denom == 0.0 {
            // Ray is parallel to plane. Hit only if origin is on the plane.
            if plane.distance_to_point(self.origin) == 0.0 {
                return Some(0.0);
            }
            return None;
        }
        let t = -(self.origin.dot(plane.normal) + plane.constant) / denom;
        if t >= 0.0 {
            Some(t)
        } else {
            None
        }
    }

    pub fn intersects_plane(&self, plane: &Plane) -> bool {
        self.intersect_plane(plane).is_some()
    }

    /// Möller–Trumbore. If `backface_culling` is true, hits with negative det are rejected.
    /// Returns the t along the ray on hit.
    pub fn intersect_triangle(&self, tri: &Triangle, backface_culling: bool) -> Option<f32> {
        let edge1 = tri.b - tri.a;
        let edge2 = tri.c - tri.a;
        let pvec = self.direction.cross(edge2);
        let det = edge1.dot(pvec);
        if backface_culling {
            if det <= 0.0 {
                return None;
            }
        } else if det.abs() < 1e-8 {
            return None;
        }
        let inv_det = 1.0 / det;
        let tvec = self.origin - tri.a;
        let u = tvec.dot(pvec) * inv_det;
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let qvec = tvec.cross(edge1);
        let v = self.direction.dot(qvec) * inv_det;
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = edge2.dot(qvec) * inv_det;
        if t < 0.0 {
            return None;
        }
        Some(t)
    }

    pub fn apply_matrix4(&self, m: &Matrix4) -> Self {
        let e = &m.elements;
        let w = e[3] * self.origin.x + e[7] * self.origin.y + e[11] * self.origin.z + e[15];
        let inv_w = if w == 0.0 { 1.0 } else { 1.0 / w };
        let origin = Vector3::new(
            (e[0] * self.origin.x + e[4] * self.origin.y + e[8] * self.origin.z + e[12]) * inv_w,
            (e[1] * self.origin.x + e[5] * self.origin.y + e[9] * self.origin.z + e[13]) * inv_w,
            (e[2] * self.origin.x + e[6] * self.origin.y + e[10] * self.origin.z + e[14]) * inv_w,
        );
        let direction = Vector3::new(
            e[0] * self.direction.x + e[4] * self.direction.y + e[8] * self.direction.z,
            e[1] * self.direction.x + e[5] * self.direction.y + e[9] * self.direction.z,
            e[2] * self.direction.x + e[6] * self.direction.y + e[10] * self.direction.z,
        )
        .normalize();
        Self { origin, direction }
    }
}

fn slab(o: f32, min: f32, max: f32, inv_d: f32) -> (f32, f32) {
    let t1 = (min - o) * inv_d;
    let t2 = (max - o) * inv_d;
    if t1 < t2 {
        (t1, t2)
    } else {
        (t2, t1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersect_unit_sphere_from_minus_z() {
        let r = Ray::new(Vector3::new(0.0, 0.0, -5.0), Vector3::new(0.0, 0.0, 1.0));
        let s = Sphere::new(Vector3::ZERO, 1.0);
        let t = r.intersect_sphere(&s).expect("hit");
        assert!((t - 4.0).abs() < 1e-5);
    }

    #[test]
    fn intersect_axis_aligned_box() {
        let r = Ray::new(Vector3::new(0.0, 0.0, -5.0), Vector3::new(0.0, 0.0, 1.0));
        let b = Box3::new(Vector3::new(-1.0, -1.0, -1.0), Vector3::new(1.0, 1.0, 1.0));
        let t = r.intersect_box(&b).expect("hit");
        assert!((t - 4.0).abs() < 1e-5);
    }

    #[test]
    fn intersect_triangle_basic() {
        let r = Ray::new(Vector3::new(0.25, 0.25, -1.0), Vector3::new(0.0, 0.0, 1.0));
        let tri = Triangle::new(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        );
        let t = r.intersect_triangle(&tri, false).expect("hit");
        assert!((t - 1.0).abs() < 1e-5);
    }
}

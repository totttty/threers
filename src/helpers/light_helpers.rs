use crate::core::{BufferAttribute, BufferGeometry, LineSegments, Object3D};
use crate::lights::{DirectionalLight, HemisphereLight, PointLight, SpotLight};
use crate::materials::LineBasicMaterial;
use crate::math::{Color, Vector3};

/// Single line from origin along the light's direction. Mirrors three.js's `DirectionalLightHelper`.
pub struct DirectionalLightHelper;

impl DirectionalLightHelper {
    pub fn new(light: &DirectionalLight, length: f32) -> Object3D {
        let d = light.direction.normalize() * length;
        let positions = vec![0.0, 0.0, 0.0, d.x, d.y, d.z];
        let mut g = BufferGeometry::new();
        g.set_attribute("position", BufferAttribute::new(positions, 3));
        Object3D::line_segments(LineSegments::new(g, LineBasicMaterial::new(light.color).into()))
    }
}

/// Octahedron at the point light's local origin. Mirrors three.js's `PointLightHelper`.
pub struct PointLightHelper;

impl PointLightHelper {
    pub fn new(light: &PointLight, size: f32) -> Object3D {
        let positions = octahedron_lines(size);
        let mut g = BufferGeometry::new();
        g.set_attribute("position", BufferAttribute::new(positions, 3));
        Object3D::line_segments(LineSegments::new(g, LineBasicMaterial::new(light.color).into()))
    }
}

/// Cone outline aligned with the spot light's direction. Mirrors three.js's `SpotLightHelper`.
pub struct SpotLightHelper;

impl SpotLightHelper {
    pub fn new(light: &SpotLight, length: f32) -> Object3D {
        let dir = light.direction.normalize();
        let tip = dir * length;
        let radius = length * light.angle.tan();
        // Find two perpendicular vectors to the direction.
        let up = if dir.y.abs() < 0.99 { Vector3::new(0.0, 1.0, 0.0) } else { Vector3::new(1.0, 0.0, 0.0) };
        let side1 = dir.cross(up).normalize();
        let side2 = dir.cross(side1).normalize();
        // Cone rim circle approximated by 16 segments.
        let mut positions = Vec::new();
        let segs = 16;
        let mut rim_pts = Vec::with_capacity(segs);
        for i in 0..segs {
            let t = i as f32 / segs as f32 * std::f32::consts::PI * 2.0;
            let p = tip + side1 * (radius * t.cos()) + side2 * (radius * t.sin());
            rim_pts.push(p);
        }
        // Rim edges.
        for i in 0..segs {
            let a = rim_pts[i];
            let b = rim_pts[(i + 1) % segs];
            positions.extend_from_slice(&[a.x, a.y, a.z, b.x, b.y, b.z]);
        }
        // Spokes from apex to rim at 4 cardinal positions.
        for i in [0, segs / 4, segs / 2, 3 * segs / 4] {
            let p = rim_pts[i];
            positions.extend_from_slice(&[0.0, 0.0, 0.0, p.x, p.y, p.z]);
        }
        let mut g = BufferGeometry::new();
        g.set_attribute("position", BufferAttribute::new(positions, 3));
        Object3D::line_segments(LineSegments::new(g, LineBasicMaterial::new(light.color).into()))
    }
}

/// Octahedron split by hemisphere — top half tinted sky color, bottom ground.
/// Mirrors three.js's `HemisphereLightHelper`.
pub struct HemisphereLightHelper;

impl HemisphereLightHelper {
    pub fn new(light: &HemisphereLight, size: f32) -> Object3D {
        // Same lines as octahedron; color picks the sky color (single-color
        // material — full per-vertex coloring needs vertex_color attributes).
        let positions = octahedron_lines(size);
        let mut g = BufferGeometry::new();
        g.set_attribute("position", BufferAttribute::new(positions, 3));
        Object3D::line_segments(LineSegments::new(g, LineBasicMaterial::new(light.sky_color).into()))
    }
}

fn octahedron_lines(size: f32) -> Vec<f32> {
    let v = [
        Vector3::new( size, 0.0, 0.0),
        Vector3::new(-size, 0.0, 0.0),
        Vector3::new(0.0,  size, 0.0),
        Vector3::new(0.0, -size, 0.0),
        Vector3::new(0.0, 0.0,  size),
        Vector3::new(0.0, 0.0, -size),
    ];
    let edges = [
        (0, 2), (0, 3), (0, 4), (0, 5),
        (1, 2), (1, 3), (1, 4), (1, 5),
        (2, 4), (4, 3), (3, 5), (5, 2),
    ];
    let mut out = Vec::with_capacity(edges.len() * 6);
    for (a, b) in edges {
        let pa = v[a]; let pb = v[b];
        out.extend_from_slice(&[pa.x, pa.y, pa.z, pb.x, pb.y, pb.z]);
    }
    out
}

/// Bone connections rendered as line segments. Mirrors three.js's `SkeletonHelper`.
pub struct SkeletonHelper;

impl SkeletonHelper {
    pub fn from_world_positions(bones_world: &[(Vector3, Option<usize>)]) -> Object3D {
        let mut positions = Vec::new();
        for (i, (p, parent)) in bones_world.iter().enumerate() {
            if let Some(parent_i) = parent {
                let pp = bones_world[*parent_i].0;
                positions.extend_from_slice(&[pp.x, pp.y, pp.z, p.x, p.y, p.z]);
            }
            let _ = i;
        }
        let mut g = BufferGeometry::new();
        g.set_attribute("position", BufferAttribute::new(positions, 3));
        Object3D::line_segments(LineSegments::new(g, LineBasicMaterial::new(Color::from_hex(0xff00ff)).into()))
    }
}

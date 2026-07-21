use crate::cameras::Camera;
use crate::core::ObjectKind;
use crate::math::Vector3;
use crate::scene::Scene;

/// Produces an SVG document by projecting triangle edges to screen space.
/// Mirrors three.js's `SVGRenderer` for line / wireframe output — fills are
/// approximated by a single per-triangle `<polygon>` element with the
/// material's base color.
pub struct SvgRenderer {
    pub width: u32,
    pub height: u32,
}

impl SvgRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn render_to_string(&self, scene: &mut Scene, camera: &dyn Camera) -> String {
        scene.update_world();
        let vp = camera.projection_matrix().multiply(&camera.view_matrix());
        let half_w = self.width as f32 * 0.5;
        let half_h = self.height as f32 * 0.5;
        let mut svg = String::new();
        svg.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
            self.width, self.height, self.width, self.height,
        ));
        let root = scene.root;
        scene.arena.traverse_visible(root, &mut |_, obj| {
            let ObjectKind::Mesh(mesh) = &obj.kind else { return; };
            let Some(iter) = mesh.geometry.positions() else { return; };
            let positions: Vec<Vector3> = iter.collect();
            let model = &obj.matrix_world;
            let project = |p: Vector3| -> (f32, f32, f32) {
                let world = transform_point(model, p);
                let clip = vp_apply(&vp, world);
                if clip[3] <= 0.0 { return (-1.0, -1.0, -1.0); }
                let nx = clip[0] / clip[3];
                let ny = clip[1] / clip[3];
                let nz = clip[2] / clip[3];
                ((nx + 1.0) * half_w, (1.0 - ny) * half_h, nz)
            };
            let triangles: Vec<[usize; 3]> = match &mesh.geometry.index {
                Some(idx) => idx.chunks_exact(3).map(|c| [c[0] as usize, c[1] as usize, c[2] as usize]).collect(),
                None => (0..positions.len() / 3).map(|i| [i * 3, i * 3 + 1, i * 3 + 2]).collect(),
            };
            let c = mesh.material.color();
            let hex = format!("rgb({},{},{})", (c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8);
            for t in triangles {
                let (a_x, a_y, a_z) = project(positions[t[0]]);
                let (b_x, b_y, b_z) = project(positions[t[1]]);
                let (cx, cy, cz) = project(positions[t[2]]);
                if a_z < -1.0 && b_z < -1.0 && cz < -1.0 { continue; }
                svg.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\" stroke=\"black\" stroke-width=\"0.3\"/>",
                    a_x, a_y, b_x, b_y, cx, cy, hex,
                ));
            }
        });
        svg.push_str("</svg>");
        svg
    }
}

fn transform_point(m: &crate::math::Matrix4, p: Vector3) -> Vector3 {
    let e = &m.elements;
    Vector3::new(
        e[0] * p.x + e[4] * p.y + e[8] * p.z + e[12],
        e[1] * p.x + e[5] * p.y + e[9] * p.z + e[13],
        e[2] * p.x + e[6] * p.y + e[10] * p.z + e[14],
    )
}

fn vp_apply(m: &crate::math::Matrix4, p: Vector3) -> [f32; 4] {
    let e = &m.elements;
    [
        e[0] * p.x + e[4] * p.y + e[8] * p.z + e[12],
        e[1] * p.x + e[5] * p.y + e[9] * p.z + e[13],
        e[2] * p.x + e[6] * p.y + e[10] * p.z + e[14],
        e[3] * p.x + e[7] * p.y + e[11] * p.z + e[15],
    ]
}

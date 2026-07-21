use crate::cameras::Camera;
use crate::math::{Vector2, Vector3};
use crate::scene::Scene;

/// Projects world-space anchor points to 2D screen-space pixel coordinates,
/// returning `(name, pixel_x, pixel_y)` triples. Mirrors three.js's
/// `CSS2DRenderer` for the part that's portable — the actual DOM element
/// layout is the integrator's responsibility.
pub struct Css2dRenderer {
    pub width: u32,
    pub height: u32,
}

impl Css2dRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn project_named_anchors(
        &self,
        scene: &mut Scene,
        camera: &dyn Camera,
    ) -> Vec<(String, Vector2)> {
        scene.update_world();
        let view = camera.view_matrix();
        let proj = camera.projection_matrix();
        let vp = proj.multiply(&view);
        let half_w = self.width as f32 * 0.5;
        let half_h = self.height as f32 * 0.5;
        let mut out = Vec::new();
        let root = scene.root;
        scene.arena.traverse_visible(root, &mut |_, obj| {
            if obj.name.is_empty() {
                return;
            }
            let world_pos = Vector3::new(
                obj.matrix_world.elements[12],
                obj.matrix_world.elements[13],
                obj.matrix_world.elements[14],
            );
            let clip = transform_vec3_to_vec4(&vp, world_pos);
            if clip[3] <= 0.0 {
                return;
            }
            let ndc_x = clip[0] / clip[3];
            let ndc_y = clip[1] / clip[3];
            let px = (ndc_x + 1.0) * half_w;
            let py = (1.0 - ndc_y) * half_h;
            out.push((obj.name.clone(), Vector2::new(px, py)));
        });
        out
    }
}

fn transform_vec3_to_vec4(m: &crate::math::Matrix4, v: Vector3) -> [f32; 4] {
    let e = &m.elements;
    [
        e[0] * v.x + e[4] * v.y + e[8] * v.z + e[12],
        e[1] * v.x + e[5] * v.y + e[9] * v.z + e[13],
        e[2] * v.x + e[6] * v.y + e[10] * v.z + e[14],
        e[3] * v.x + e[7] * v.y + e[11] * v.z + e[15],
    ]
}

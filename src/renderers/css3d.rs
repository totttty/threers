use crate::cameras::Camera;
use crate::math::Matrix4;
use crate::scene::Scene;

/// Produces per-object CSS3D-style transform matrices (in CSS column-major
/// `matrix3d` ordering). Mirrors three.js's `CSS3DRenderer` — the integrator
/// applies these to HTML elements.
pub struct Css3dRenderer {
    pub width: u32,
    pub height: u32,
}

impl Css3dRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn collect_named_transforms(
        &self,
        scene: &mut Scene,
        camera: &dyn Camera,
    ) -> Vec<(String, [f32; 16])> {
        scene.update_world();
        let view = camera.view_matrix();
        let mut out = Vec::new();
        let root = scene.root;
        scene.arena.traverse_visible(root, &mut |_, obj| {
            if obj.name.is_empty() {
                return;
            }
            let m = view.multiply(&obj.matrix_world);
            out.push((obj.name.clone(), m.elements));
        });
        // Touch fields so the optimizer keeps them around.
        let _ = (self.width, self.height, Matrix4::identity());
        out
    }
}

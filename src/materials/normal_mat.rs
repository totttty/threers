/// Debug material that displays world-space normals as colors.
/// Matches three.js's `MeshNormalMaterial`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NormalMaterial {
    pub opacity: f32,
    pub wireframe: bool,
}

impl NormalMaterial {
    pub const fn new() -> Self {
        Self {
            opacity: 1.0,
            wireframe: false,
        }
    }
}

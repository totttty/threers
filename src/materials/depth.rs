/// Debug material that displays linearized depth as grayscale.
/// Matches three.js's `MeshDepthMaterial`.
#[derive(Debug, Clone, Copy)]
pub struct DepthMaterial {
    pub opacity: f32,
    pub near: f32,
    pub far: f32,
    pub wireframe: bool,
}

impl Default for DepthMaterial {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            near: 0.1,
            far: 100.0,
            wireframe: false,
        }
    }
}

impl DepthMaterial {
    pub fn new() -> Self {
        Self::default()
    }
}

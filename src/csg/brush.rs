//! CSG brush: geometry + world matrix + BVH / half-edges for boolean ops.
//!
//! Mirrors `web/csg/core/Brush.js`.

use crate::core::BufferGeometry;
use crate::math::{Matrix4, Vector3};
use crate::mesh_bvh::{BuildOptions, MeshBvh};

use super::geometry_prep::{build_group_indices, ensure_index, ensure_non_indexed};
use super::half_edge_map::HalfEdgeMap;
use super::js_topology::JsMatrix4;

/// A mesh operand for CSG evaluation.
#[derive(Debug)]
pub struct CsgBrush {
    pub geometry: BufferGeometry,
    pub local_matrix: Matrix4,
    pub matrix_world: Matrix4,
    previous_local_matrix: Matrix4,
    position_js: Option<[f64; 3]>,
    pub(crate) half_edges: Option<HalfEdgeMap>,
    pub(crate) group_indices: Vec<u16>,
}

impl CsgBrush {
    pub fn new(geometry: BufferGeometry) -> Self {
        let mut previous_local_matrix = Matrix4::identity();
        previous_local_matrix.elements.fill(0.0);
        Self {
            geometry,
            local_matrix: Matrix4::identity(),
            matrix_world: Matrix4::identity(),
            previous_local_matrix,
            position_js: None,
            half_edges: None,
            group_indices: Vec::new(),
        }
    }

    /// Set brush position (f64 literals match three.js `position.set`).
    pub fn set_position(&mut self, x: f64, y: f64, z: f64) {
        self.local_matrix = Matrix4::translation(Vector3::new(x as f32, y as f32, z as f32));
        self.position_js = Some([x, y, z]);
    }

    /// `matrixWorld` as f64 translation for CSG parity (`threejs-shim.js`).
    pub(crate) fn js_matrix_world(&self) -> JsMatrix4 {
        if let Some([x, y, z]) = self.position_js {
            let mut m = JsMatrix4::identity();
            m.elements[12] = x;
            m.elements[13] = y;
            m.elements[14] = z;
            m
        } else {
            JsMatrix4::from_matrix4(&self.matrix_world)
        }
    }

    pub fn update_matrix_world(&mut self, parent: Option<&Matrix4>) {
        self.matrix_world = parent
            .map(|p| p.multiply(&self.local_matrix))
            .unwrap_or(self.local_matrix);
    }

    pub fn is_dirty(&self) -> bool {
        self.local_matrix.elements != self.previous_local_matrix.elements
    }

    pub fn mark_updated(&mut self) {
        self.previous_local_matrix = self.local_matrix;
    }

    /// Build index, BVH, and half-edge map (required before evaluate).
    pub fn prepare_geometry(&mut self) {
        // Primitives arrive as non-indexed soup in JS (`geometryToBufferGeometry` →
        // `toNonIndexed`); mirror that before `ensureIndex`.
        ensure_non_indexed(&mut self.geometry);
        ensure_index(&mut self.geometry);
        if self.geometry.bounds_tree.is_none() {
            let opts = BuildOptions {
                max_leaf_tris: 3,
                indirect: true,
                ..Default::default()
            };
            self.geometry.compute_bounds_tree(opts);
        }
        self.half_edges = Some(HalfEdgeMap::from_geometry(&self.geometry));
        self.group_indices = build_group_indices(&self.geometry);
    }

    pub fn bounds_tree(&self) -> std::sync::Arc<MeshBvh> {
        self.geometry
            .bounds_tree
            .as_ref()
            .expect("bounds tree")
            .clone()
    }
}

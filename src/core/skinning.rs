use std::sync::Arc;
use crate::math::Matrix4;
use super::{BufferAttribute, BufferGeometry, ObjectId};
use crate::materials::Material;

/// A bone identified by its node id + the inverse-bind matrix that maps mesh
/// vertices into bone-local space. Mirrors three.js's `Bone` plus
/// `Skeleton.boneInverses[i]`.
#[derive(Debug, Clone, Copy)]
pub struct Bone {
    pub node: ObjectId,
    pub inverse_bind: Matrix4,
}

/// Collection of bones plus their world matrices (updated each frame).
/// The renderer reads `bone_matrices` via a storage buffer.
#[derive(Debug, Clone, Default)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
    /// Final per-bone matrix = node.matrix_world * inverse_bind. Caller
    /// (typically the user's animation loop or `Skeleton::update`) is
    /// responsible for writing this.
    pub bone_matrices: Vec<Matrix4>,
}

impl Skeleton {
    pub fn new(bones: Vec<Bone>) -> Self {
        let count = bones.len();
        Self { bones, bone_matrices: vec![Matrix4::identity(); count] }
    }

    /// Recompute every `bone_matrices[i]` from the corresponding node's
    /// current `matrix_world` and the bone's `inverse_bind`. Mirrors
    /// three.js's `Skeleton.update()`.
    pub fn update(&mut self, arena: &super::ObjectArena) {
        if self.bone_matrices.len() != self.bones.len() {
            self.bone_matrices.resize(self.bones.len(), Matrix4::identity());
        }
        for (i, bone) in self.bones.iter().enumerate() {
            let world = arena.get(bone.node)
                .map(|o| o.matrix_world)
                .unwrap_or_else(Matrix4::identity);
            self.bone_matrices[i] = world.multiply(&bone.inverse_bind);
        }
    }
}

/// Mesh that follows a skeleton. The geometry must carry `joint` (vec4 of bone
/// indices) and `weight` (vec4 of bone weights) attributes.
#[derive(Debug, Clone)]
pub struct SkinnedMesh {
    pub geometry: Arc<BufferGeometry>,
    pub material: Arc<Material>,
    pub skeleton: Arc<Skeleton>,
}

impl SkinnedMesh {
    pub fn new(geometry: BufferGeometry, material: Material, skeleton: Skeleton) -> Self {
        Self {
            geometry: Arc::new(geometry),
            material: Arc::new(material),
            skeleton: Arc::new(skeleton),
        }
    }

    /// Convenience: build a skinned mesh from a non-skinned `BufferGeometry`
    /// by adding zeroed joint/weight attributes (every vertex bound to bone 0
    /// with weight 1).
    pub fn from_unweighted(mut geometry: BufferGeometry, material: Material, skeleton: Skeleton) -> Self {
        let count = geometry.get_attribute("position").map(|a| a.count()).unwrap_or(0);
        if geometry.get_attribute("joint").is_none() {
            geometry.set_attribute("joint", BufferAttribute::new(vec![0.0; count * 4], 4));
        }
        if geometry.get_attribute("weight").is_none() {
            let mut weights = vec![0.0; count * 4];
            for chunk in weights.chunks_exact_mut(4) { chunk[0] = 1.0; }
            geometry.set_attribute("weight", BufferAttribute::new(weights, 4));
        }
        Self::new(geometry, material, skeleton)
    }
}

/// One animatable per-vertex offset target (a "morph"). Mirrors three.js's
/// morph-target data. Stored as deltas relative to the base position.
#[derive(Debug, Clone, Default)]
pub struct MorphTarget {
    pub name: String,
    /// Per-vertex position deltas (length = vertex_count * 3).
    pub position_delta: Vec<f32>,
    /// Optional per-vertex normal deltas.
    pub normal_delta: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Default)]
pub struct MorphAttributes {
    pub targets: Vec<MorphTarget>,
    pub influences: Vec<f32>,
}

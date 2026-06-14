use crate::core::{ObjectArena, ObjectId};
use crate::math::{Quaternion, Vector3};

/// One bone in a CCD IK chain.
#[derive(Debug, Clone, Copy)]
pub struct IkBone {
    pub node: ObjectId,
    pub limit_min: Option<f32>,
    pub limit_max: Option<f32>,
}

/// Cyclic Coordinate Descent inverse-kinematics solver. Mirrors three.js's
/// `CCDIKSolver` for one chain. The solver iterates from the tip back to the
/// root, rotating each bone so its tip aligns toward the goal.
pub struct CcdIkSolver {
    pub chain: Vec<IkBone>,
    /// Target position in world space.
    pub goal: Vector3,
    pub iterations: u32,
    pub tolerance: f32,
}

impl CcdIkSolver {
    pub fn new(chain: Vec<IkBone>, goal: Vector3) -> Self {
        Self { chain, goal, iterations: 16, tolerance: 1e-3 }
    }

    pub fn solve(&self, arena: &mut ObjectArena) {
        if self.chain.len() < 2 { return; }
        let tip = self.chain[self.chain.len() - 1].node;
        for _ in 0..self.iterations {
            for i in (0..self.chain.len() - 1).rev() {
                let bone = self.chain[i];
                let bone_pos = world_pos(arena, bone.node);
                let tip_pos = world_pos(arena, tip);
                let to_tip = (tip_pos - bone_pos).normalize();
                let to_goal = (self.goal - bone_pos).normalize();
                let axis = to_tip.cross(to_goal);
                let dot = to_tip.dot(to_goal).clamp(-1.0, 1.0);
                let angle = dot.acos();
                if angle < 1e-5 { continue; }
                let axis_len = axis.length();
                if axis_len < 1e-6 { continue; }
                let q = Quaternion::from_axis_angle(axis * (1.0 / axis_len), angle * 0.5);
                if let Some(obj) = arena.get_mut(bone.node) {
                    obj.quaternion = q.multiply(obj.quaternion);
                    if let Some(lo) = bone.limit_min {
                        let _ = lo;
                    }
                }
            }
            arena.update_world_matrices(self.chain[0].node, crate::math::Matrix4::identity());
            let tip_pos = world_pos(arena, tip);
            if (self.goal - tip_pos).length_sq() < self.tolerance * self.tolerance {
                break;
            }
        }
    }
}

fn world_pos(arena: &ObjectArena, id: ObjectId) -> Vector3 {
    arena.get(id).map(|o| {
        let e = &o.matrix_world.elements;
        Vector3::new(e[12], e[13], e[14])
    }).unwrap_or(Vector3::ZERO)
}

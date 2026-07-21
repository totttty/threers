use super::track::TrackTarget;
use super::{AnimationAction, AnimationClip};
use crate::core::ObjectArena;
use crate::materials::Material;
use crate::scene::Scene;

/// Plays animation clips against a scene. Holds zero or more `AnimationAction`s.
pub struct AnimationMixer {
    pub actions: Vec<AnimationAction>,
    pub time: f32,
}

impl Default for AnimationMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationMixer {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            time: 0.0,
        }
    }

    pub fn clip_action(&mut self, clip: AnimationClip) -> usize {
        let idx = self.actions.len();
        self.actions.push(AnimationAction::new(clip));
        idx
    }

    /// Advance time and apply every active action's tracks to the scene.
    pub fn update(&mut self, scene: &mut Scene, delta: f32) {
        self.time += delta;
        for action in &mut self.actions {
            if !action.enabled {
                continue;
            }
            action.advance(delta);
            apply_action(&mut scene.arena, action);
        }
    }
}

fn apply_action(arena: &mut ObjectArena, action: &AnimationAction) {
    let t = action.current_time();
    for tr in &action.clip.tracks {
        let Some(obj) = arena.nodes.get_mut(tr.object) else {
            continue;
        };
        match tr.target {
            TrackTarget::Position => {
                if let Some(v) = tr.sample_vector(t) {
                    obj.position = v;
                }
            }
            TrackTarget::Quaternion => {
                if let Some(q) = tr.sample_quaternion(t) {
                    obj.quaternion = q;
                }
            }
            TrackTarget::Scale => {
                if let Some(v) = tr.sample_vector(t) {
                    obj.scale = v;
                }
            }
            TrackTarget::Color => {
                if let Some(c) = tr.sample_color(t) {
                    if let crate::core::ObjectKind::Mesh(mesh) = &mut obj.kind {
                        if let Some(mat) = std::sync::Arc::get_mut(&mut mesh.material) {
                            apply_color(mat, c);
                        }
                    }
                }
            }
            TrackTarget::Scalar => { /* user-driven; nothing to apply */ }
        }
    }
}

fn apply_color(mat: &mut Material, c: crate::math::Color) {
    match mat {
        Material::Basic(m) => m.color = c,
        Material::Lambert(m) => m.color = c,
        Material::Phong(m) => m.color = c,
        Material::Standard(m) => m.color = c,
        Material::Physical(m) => m.color = c,
        Material::Toon(m) => m.color = c,
        Material::Matcap(m) => m.color = c,
        Material::Line(m) => m.color = c,
        Material::Points(m) => m.color = c,
        Material::Sprite(m) => m.color = c,
        Material::Mirror(m) => m.color = c,
        Material::Normal(_) | Material::Depth(_) | Material::Distance(_) | Material::Sky(_) => {}
        Material::Shader(_) => {}
    }
}

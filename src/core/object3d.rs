use slotmap::{new_key_type, SlotMap};
use crate::math::{Vector3, Quaternion, Matrix4};
use crate::lights::Light;
use super::{Mesh, Layers, LineSegments, Points, Sprite, InstancedMesh, SkinnedMesh};

new_key_type! {
    /// Stable handle to an Object3D in a Scene. Cloneable, `Copy`.
    pub struct ObjectId;
}

/// What kind of node this is. Groups have no geometry; meshes do.
/// Cameras live in their own types — this enum is for the renderable graph.
#[derive(Debug, Clone)]
pub enum ObjectKind {
    Group,
    Mesh(Mesh),
    Light(Light),
    LineSegments(LineSegments),
    Points(Points),
    Sprite(Sprite),
    InstancedMesh(InstancedMesh),
    SkinnedMesh(SkinnedMesh),
}

/// A node in the scene graph. Matches three.js's `Object3D`: position,
/// rotation (as a quaternion), scale, plus parent/child links and a world
/// matrix updated by the renderer.
#[derive(Debug, Clone)]
pub struct Object3D {
    pub name: String,
    pub position: Vector3,
    pub quaternion: Quaternion,
    pub scale: Vector3,
    pub up: Vector3,
    pub visible: bool,
    pub layers: Layers,
    /// three.js `Object3D.castShadow` — included in shadow-map depth passes.
    pub cast_shadow: bool,
    /// three.js `Object3D.receiveShadow` — directional/spot/point shadows darken this mesh.
    pub receive_shadow: bool,
    /// three.js `Object3D.renderOrder` — lower values draw first within the same transparency class.
    pub render_order: i32,
    pub kind: ObjectKind,

    pub matrix: Matrix4,
    pub matrix_world: Matrix4,

    pub parent: Option<ObjectId>,
    pub children: Vec<ObjectId>,
}

impl Object3D {
    pub fn group() -> Self {
        Self::with_kind(ObjectKind::Group)
    }

    pub fn mesh(mesh: Mesh) -> Self {
        Self::with_kind(ObjectKind::Mesh(mesh))
    }

    pub fn light(light: impl Into<Light>) -> Self {
        Self::with_kind(ObjectKind::Light(light.into()))
    }

    pub fn line_segments(ls: LineSegments) -> Self {
        Self::with_kind(ObjectKind::LineSegments(ls))
    }

    pub fn points(p: Points) -> Self {
        Self::with_kind(ObjectKind::Points(p))
    }

    pub fn sprite(s: Sprite) -> Self {
        Self::with_kind(ObjectKind::Sprite(s))
    }

    pub fn instanced_mesh(im: InstancedMesh) -> Self {
        Self::with_kind(ObjectKind::InstancedMesh(im))
    }

    pub fn skinned_mesh(sm: SkinnedMesh) -> Self {
        Self::with_kind(ObjectKind::SkinnedMesh(sm))
    }

    fn with_kind(kind: ObjectKind) -> Self {
        Self {
            name: String::new(),
            position: Vector3::ZERO,
            quaternion: Quaternion::identity(),
            scale: Vector3::ONE,
            up: Vector3::UP,
            visible: true,
            layers: Layers::default(),
            cast_shadow: false,
            receive_shadow: false,
            render_order: 0,
            kind,
            matrix: Matrix4::identity(),
            matrix_world: Matrix4::identity(),
            parent: None,
            children: Vec::new(),
        }
    }

    /// Recompute local matrix from position/quaternion/scale.
    pub fn update_matrix(&mut self) {
        self.matrix = Matrix4::compose(self.position, self.quaternion, self.scale);
    }

    // -- transformation helpers (matches three.js `Object3D`) --

    /// Translate by `distance` along an axis given in **local** space.
    pub fn translate_on_axis(&mut self, axis: Vector3, distance: f32) -> &mut Self {
        let world_axis = axis.apply_quaternion(self.quaternion);
        self.position = self.position + world_axis * distance;
        self
    }

    pub fn translate_x(&mut self, d: f32) -> &mut Self {
        self.translate_on_axis(Vector3::new(1.0, 0.0, 0.0), d)
    }

    pub fn translate_y(&mut self, d: f32) -> &mut Self {
        self.translate_on_axis(Vector3::new(0.0, 1.0, 0.0), d)
    }

    pub fn translate_z(&mut self, d: f32) -> &mut Self {
        self.translate_on_axis(Vector3::new(0.0, 0.0, 1.0), d)
    }

    /// Rotate about a unit axis given in **local** space.
    pub fn rotate_on_axis(&mut self, axis: Vector3, angle: f32) -> &mut Self {
        let q = Quaternion::from_axis_angle(axis, angle);
        self.quaternion = self.quaternion.multiply(q);
        self
    }

    /// Rotate about a unit axis given in **world** space.
    pub fn rotate_on_world_axis(&mut self, axis: Vector3, angle: f32) -> &mut Self {
        let q = Quaternion::from_axis_angle(axis, angle);
        self.quaternion = self.quaternion.premultiply(q);
        self
    }

    pub fn rotate_x(&mut self, angle: f32) -> &mut Self {
        self.rotate_on_axis(Vector3::new(1.0, 0.0, 0.0), angle)
    }

    pub fn rotate_y(&mut self, angle: f32) -> &mut Self {
        self.rotate_on_axis(Vector3::new(0.0, 1.0, 0.0), angle)
    }

    pub fn rotate_z(&mut self, angle: f32) -> &mut Self {
        self.rotate_on_axis(Vector3::new(0.0, 0.0, 1.0), angle)
    }

    /// Pre-multiply the local matrix by `m`, then redecompose into position/quaternion/scale.
    pub fn apply_matrix4(&mut self, m: &Matrix4) -> &mut Self {
        self.update_matrix();
        let composed = m.multiply(&self.matrix);
        let (p, q, s) = composed.decompose();
        self.position = p;
        self.quaternion = q;
        self.scale = s;
        self
    }

    /// Set the object's rotation so its local -Z axis points at `target` in world space.
    /// **Does not handle non-trivial parent rotations.** For parented objects, ensure
    /// the parent's `matrix_world` is identity, or compose the inverse-parent rotation
    /// after calling this.
    pub fn look_at(&mut self, target: Vector3) -> &mut Self {
        let m = Matrix4::target_look_at(self.position, target, self.up);
        self.quaternion = Quaternion::from_rotation_matrix(&m);
        self
    }

    pub fn world_position(&self) -> Vector3 {
        let e = &self.matrix_world.elements;
        Vector3::new(e[12], e[13], e[14])
    }

    pub fn world_quaternion(&self) -> Quaternion {
        let (_, q, _) = self.matrix_world.decompose();
        q
    }

    pub fn world_scale(&self) -> Vector3 {
        let (_, _, s) = self.matrix_world.decompose();
        s
    }
}

/// Arena that owns all `Object3D`s in a graph and exposes three.js-style
/// `add`/`remove`/`traverse`. Roots are tracked separately so the renderer
/// can walk from any root.
#[derive(Debug, Default)]
pub struct ObjectArena {
    pub nodes: SlotMap<ObjectId, Object3D>,
}

impl ObjectArena {
    pub fn new() -> Self { Self::default() }

    pub fn insert(&mut self, obj: Object3D) -> ObjectId {
        self.nodes.insert(obj)
    }

    pub fn get(&self, id: ObjectId) -> Option<&Object3D> {
        self.nodes.get(id)
    }

    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut Object3D> {
        self.nodes.get_mut(id)
    }

    /// Attach `child` under `parent`. If `child` already has a parent, it is
    /// detached first — matches three.js semantics.
    pub fn add_child(&mut self, parent: ObjectId, child: ObjectId) {
        if let Some(old_parent) = self.nodes.get(child).and_then(|c| c.parent) {
            if let Some(p) = self.nodes.get_mut(old_parent) {
                p.children.retain(|c| *c != child);
            }
        }
        if let Some(c) = self.nodes.get_mut(child) {
            c.parent = Some(parent);
        }
        if let Some(p) = self.nodes.get_mut(parent) {
            p.children.push(child);
        }
    }

    /// Detach `child` from its parent.
    pub fn remove_child(&mut self, child: ObjectId) {
        let parent = self.nodes.get(child).and_then(|c| c.parent);
        if let Some(p) = parent {
            if let Some(parent_obj) = self.nodes.get_mut(p) {
                parent_obj.children.retain(|c| *c != child);
            }
        }
        if let Some(c) = self.nodes.get_mut(child) {
            c.parent = None;
        }
    }

    /// Walk the subtree rooted at `id`, updating each node's `matrix_world`.
    pub fn update_world_matrices(&mut self, id: ObjectId, parent_world: Matrix4) {
        let world = {
            let Some(node) = self.nodes.get_mut(id) else { return; };
            node.update_matrix();
            node.matrix_world = parent_world.multiply(&node.matrix);
            node.matrix_world
        };
        let children = self.nodes[id].children.clone();
        for c in children {
            self.update_world_matrices(c, world);
        }
    }

    /// Pre-order traverse. Calls `f(id)` for `id` and every descendant.
    pub fn traverse(&self, id: ObjectId, f: &mut impl FnMut(ObjectId, &Object3D)) {
        if let Some(node) = self.nodes.get(id) {
            f(id, node);
            for c in node.children.clone() {
                self.traverse(c, f);
            }
        }
    }

    /// Pre-order traverse, skipping subtrees whose root is not visible.
    pub fn traverse_visible(&self, id: ObjectId, f: &mut impl FnMut(ObjectId, &Object3D)) {
        if let Some(node) = self.nodes.get(id) {
            if !node.visible { return; }
            f(id, node);
            for c in node.children.clone() {
                self.traverse_visible(c, f);
            }
        }
    }

    /// Walk up the chain from `id` through its ancestors.
    pub fn traverse_ancestors(&self, id: ObjectId, f: &mut impl FnMut(ObjectId, &Object3D)) {
        let mut cur = self.nodes.get(id).and_then(|n| n.parent);
        while let Some(p) = cur {
            if let Some(n) = self.nodes.get(p) {
                f(p, n);
                cur = n.parent;
            } else {
                break;
            }
        }
    }

    /// First object in `root`'s subtree whose name matches.
    pub fn get_object_by_name(&self, root: ObjectId, name: &str) -> Option<ObjectId> {
        let mut found = None;
        self.traverse(root, &mut |id, obj| {
            if found.is_none() && obj.name == name { found = Some(id); }
        });
        found
    }

    /// All objects in `root`'s subtree whose name matches.
    pub fn get_objects_by_name(&self, root: ObjectId, name: &str) -> Vec<ObjectId> {
        let mut out = Vec::new();
        self.traverse(root, &mut |id, obj| {
            if obj.name == name { out.push(id); }
        });
        out
    }
}

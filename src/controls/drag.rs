use crate::cameras::Camera;
use crate::core::{ObjectArena, ObjectId, Raycaster};
use crate::math::{Vector2, Vector3};
use super::PointerEvent;

/// Pointer-driven drag. On pointer down with `rotating` true (left mouse), a
/// ray is cast through the scene; if it hits one of the `draggable` objects,
/// subsequent pointer motion translates that object in the plane perpendicular
/// to the camera-forward axis at the hit depth. Mirrors three.js's `DragControls`.
pub struct DragControls {
    pub draggable: Vec<ObjectId>,
    pub active: Option<ObjectId>,
    drag_plane_depth: f32,
    last_ndc: Vector2,
}

impl DragControls {
    pub fn new(draggable: Vec<ObjectId>) -> Self {
        Self { draggable, active: None, drag_plane_depth: 0.0, last_ndc: Vector2::ZERO }
    }

    /// Call when a pointer button transitions to "down" at NDC coordinate `ndc`.
    pub fn pointer_down(&mut self, arena: &ObjectArena, ndc: Vector2, camera: &dyn Camera) {
        let mut rc = Raycaster::default();
        rc.set_from_camera_perspective(ndc, camera);
        // Caller passes a flat list; we iterate intersect_objects for each candidate.
        for id in &self.draggable {
            let hits = rc.intersect_objects(arena, *id, false);
            if let Some(h) = hits.first() {
                self.active = Some(*id);
                self.drag_plane_depth = (h.point - camera.position()).length();
                self.last_ndc = ndc;
                return;
            }
        }
    }

    pub fn pointer_up(&mut self) {
        self.active = None;
    }

    /// Convert NDC drag delta into a world translation applied to the active object.
    /// Returns the delta the caller should add to `obj.position`.
    pub fn pointer_move(&mut self, ndc: Vector2, camera: &dyn Camera, ev: PointerEvent) -> Option<(ObjectId, Vector3)> {
        let _ = ev;
        let id = self.active?;
        let view = camera.view_matrix();
        let proj = camera.projection_matrix();
        let prev = Vector3::new(self.last_ndc.x, self.last_ndc.y, 0.5).unproject(&view, &proj);
        let cur = Vector3::new(ndc.x, ndc.y, 0.5).unproject(&view, &proj);
        let cam_pos = camera.position();
        let d_prev = ((prev - cam_pos).normalize() * self.drag_plane_depth) + cam_pos;
        let d_cur = ((cur - cam_pos).normalize() * self.drag_plane_depth) + cam_pos;
        self.last_ndc = ndc;
        Some((id, d_cur - d_prev))
    }
}

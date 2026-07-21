//! Incremental builders for the `bvh-csg-hierarchy` parity scene (s1–s4).
//!
//! Live Rust CSG matches JS reference counts and ordered TriKeys exactly:
//! s1=72, s2=53070, s3=55914, s4=59454 verts.

use crate::core::BufferGeometry;
use crate::geometries::{BoxGeometry, SphereGeometry};
use crate::math::Matrix4;
use crate::utils::compute_vertex_normals;

use super::brush::CsgBrush;
use super::constants::{ADDITION, SUBTRACTION};
use super::evaluator::CsgEvaluator;
use super::geometry_prep::load_positions_geometry_bin;

/// Vertex counts from JS CSG (`threers-bvh-csg-hierarchy.html` / `web/csg/`).
pub const JS_STEP1_VERTS: usize = 72;
pub const JS_STEP2_VERTS: usize = 53_070;
pub const JS_STEP3_VERTS: usize = 55_914;
pub const JS_STEP4_VERTS: usize = 59_454;

pub const STEP1_TOLERANCE: u64 = 0;
pub const STEP2_TOLERANCE: u64 = 0;
pub const STEP3_TOLERANCE: u64 = 0;
pub const STEP4_TOLERANCE: u64 = 0;

fn brush_from_result(geom: BufferGeometry) -> CsgBrush {
    let mut b = CsgBrush::new(geom);
    b.matrix_world = Matrix4::identity();
    b
}

fn window_cut_brush() -> CsgBrush {
    let mut b = CsgBrush::new(BoxGeometry::new(1.2, 1.0, 0.5));
    b.set_position(0.8, 0.15, 1.35);
    b.update_matrix_world(None);
    b
}

fn window_frame_brush() -> CsgBrush {
    let mut b = CsgBrush::new(BoxGeometry::new(1.2, 1.0, 0.12));
    b.set_position(0.8, 0.15, 1.35);
    b.update_matrix_world(None);
    b
}

fn sphere_brush() -> CsgBrush {
    let mut b = CsgBrush::new(SphereGeometry::new(0.55, 24, 12));
    b.set_position(-1.1, 0.2, 1.35);
    b.update_matrix_world(None);
    b
}

/// **s1** — hollow shell: outer 4×2.5×2.5 − inner 3.6×2.1×2.1.
pub fn step1_shell_cut(evaluator: &mut CsgEvaluator) -> BufferGeometry {
    let mut outer = CsgBrush::new(BoxGeometry::new(4.0, 2.5, 2.5));
    let mut inner = CsgBrush::new(BoxGeometry::new(3.6, 2.1, 2.1));
    evaluator.evaluate(&mut outer, &mut inner, SUBTRACTION)
}

/// **s2** — s1 + sphere at (−1.1, 0.2, 1.35).
pub fn step2_add_sphere(evaluator: &mut CsgEvaluator, shell: BufferGeometry) -> BufferGeometry {
    let mut acc = brush_from_result(shell);
    let mut sphere = sphere_brush();
    evaluator.evaluate(&mut acc, &mut sphere, ADDITION)
}

/// **s3** — s2 − window cut box 1.2×1.0×0.5 at (0.8, 0.15, 1.35).
pub fn step3_win_cut(evaluator: &mut CsgEvaluator, after_sphere: BufferGeometry) -> BufferGeometry {
    let mut acc = brush_from_result(after_sphere);
    let mut win_cut = window_cut_brush();
    evaluator.evaluate(&mut acc, &mut win_cut, SUBTRACTION)
}

/// **s4** — s3 + thin window frame 1.2×1.0×0.12 at (0.8, 0.15, 1.35).
pub fn step4_win_frame(
    evaluator: &mut CsgEvaluator,
    after_win_cut: BufferGeometry,
) -> BufferGeometry {
    let mut acc = brush_from_result(after_win_cut);
    let mut win_frame = window_frame_brush();
    let mut geom = evaluator.evaluate(&mut acc, &mut win_frame, ADDITION);
    compute_vertex_normals(&mut geom);
    geom
}

/// Load JS-exported after-sphere soup (`bvh-csg-after-sphere.geom.bin`).
pub fn load_reference_after_sphere(raw: &[u8]) -> BufferGeometry {
    let mut geom = load_positions_geometry_bin(raw);
    compute_vertex_normals(&mut geom);
    geom
}

/// Evaluate steps 1..=`step` with live Rust CSG (no reference bins).
pub fn evaluate_live_through(step: u8, evaluator: &mut CsgEvaluator) -> BufferGeometry {
    let s1 = step1_shell_cut(evaluator);
    if step == 1 {
        return s1;
    }
    let s2 = step2_add_sphere(evaluator, s1);
    if step == 2 {
        return s2;
    }
    let s3 = step3_win_cut(evaluator, s2);
    if step == 3 {
        return s3;
    }
    step4_win_frame(evaluator, s3)
}

/// Evaluate through `step`, using a reference after-sphere bin from step 3 onward when provided.
pub fn evaluate_through(
    step: u8,
    evaluator: &mut CsgEvaluator,
    reference_after_sphere: Option<&[u8]>,
) -> BufferGeometry {
    match step {
        1 => step1_shell_cut(evaluator),
        2 => {
            let s1 = step1_shell_cut(evaluator);
            step2_add_sphere(evaluator, s1)
        }
        3 => {
            let after_sphere = reference_after_sphere
                .map(load_reference_after_sphere)
                .unwrap_or_else(|| {
                    let s1 = step1_shell_cut(evaluator);
                    step2_add_sphere(evaluator, s1)
                });
            step3_win_cut(evaluator, after_sphere)
        }
        4 => {
            let after_sphere = reference_after_sphere
                .map(load_reference_after_sphere)
                .unwrap_or_else(|| {
                    let s1 = step1_shell_cut(evaluator);
                    step2_add_sphere(evaluator, s1)
                });
            let after_cut = step3_win_cut(evaluator, after_sphere);
            step4_win_frame(evaluator, after_cut)
        }
        _ => panic!("hierarchy step must be 1..=4, got {step}"),
    }
}

pub fn js_target_verts(step: u8) -> usize {
    match step {
        1 => JS_STEP1_VERTS,
        2 => JS_STEP2_VERTS,
        3 => JS_STEP3_VERTS,
        4 => JS_STEP4_VERTS,
        _ => panic!("hierarchy step must be 1..=4, got {step}"),
    }
}

pub fn step_tolerance(step: u8) -> u64 {
    match step {
        1 => STEP1_TOLERANCE,
        2 => STEP2_TOLERANCE,
        3 => STEP3_TOLERANCE,
        4 => STEP4_TOLERANCE,
        _ => panic!("hierarchy step must be 1..=4, got {step}"),
    }
}

pub fn assert_step_verts(step: u8, actual: usize) {
    let expected = js_target_verts(step);
    let delta = (actual as i64 - expected as i64).unsigned_abs();
    let tol = step_tolerance(step);
    assert!(
        delta <= tol,
        "step {step}: {actual} verts vs JS {expected} (delta {delta} > {tol})"
    );
}

use crate::math::{Box3, Matrix4};

use super::mesh_bvh::MeshBvh;

/// Dual-BVH traversal matching upstream three-mesh-bvh `cast/bvhcast.js`.
/// Returns BVH-layout triangle index pairs `(ia, ib)` for geometry A and B.
pub fn bvhcast(a: &MeshBvh, b: &MeshBvh, matrix_to_local: &Matrix4) -> Vec<(usize, usize)> {
    if a.node_count() == 0 || b.node_count() == 0 {
        return Vec::new();
    }
    let mat_b_to_a = *matrix_to_local;
    let mat_a_to_b = matrix_to_local.invert();
    let curr_box = a.node_bounds(0).apply_matrix4(&mat_a_to_b);
    let mut pairs = Vec::new();
    traverse(
        a,
        b,
        0,
        0,
        mat_b_to_a,
        mat_a_to_b,
        &mut pairs,
        curr_box,
        false,
    );
    pairs
}

fn traverse(
    a: &MeshBvh,
    b: &MeshBvh,
    node_a: u32,
    node_b: u32,
    mat_2_to_1: Matrix4,
    mat_1_to_2: Matrix4,
    pairs: &mut Vec<(usize, usize)>,
    curr_box: Box3,
    reversed: bool,
) {
    let (s1, s2, n1, n2) = if reversed {
        (b, a, node_b, node_a)
    } else {
        (a, b, node_a, node_b)
    };

    let leaf1 = s1.is_leaf(n1);
    let leaf2 = s2.is_leaf(n2);

    if leaf1 && leaf2 {
        let (offset_a, count_a) = if reversed {
            s2.leaf_range(n2)
        } else {
            s1.leaf_range(n1)
        };
        let (offset_b, count_b) = if reversed {
            s1.leaf_range(n1)
        } else {
            s2.leaf_range(n2)
        };
        collect_leaf_pairs(offset_a, count_a, offset_b, count_b, pairs);
        return;
    }

    if leaf2 {
        let new_box = s2.node_bounds(n2).apply_matrix4(&mat_2_to_1);
        let cl = s1.child_left(n1);
        let cr = s1.child_right(n1);
        if new_box.intersects_box(&s1.node_bounds(cl)) {
            traverse(
                a,
                b,
                if reversed { node_a } else { cl },
                if reversed { cl } else { node_b },
                mat_1_to_2,
                mat_2_to_1,
                pairs,
                new_box,
                !reversed,
            );
        }
        if new_box.intersects_box(&s1.node_bounds(cr)) {
            traverse(
                a,
                b,
                if reversed { node_a } else { cr },
                if reversed { cr } else { node_b },
                mat_1_to_2,
                mat_2_to_1,
                pairs,
                new_box,
                !reversed,
            );
        }
        return;
    }

    let cl2 = s2.child_left(n2);
    let cr2 = s2.child_right(n2);
    let left_box2 = s2.node_bounds(cl2);
    let right_box2 = s2.node_bounds(cr2);
    let left_hit = curr_box.intersects_box(&left_box2);
    let right_hit = curr_box.intersects_box(&right_box2);

    if left_hit && right_hit {
        traverse(
            a,
            b,
            node_a,
            cl2,
            mat_2_to_1,
            mat_1_to_2,
            pairs,
            curr_box,
            reversed,
        );
        traverse(
            a,
            b,
            node_a,
            cr2,
            mat_2_to_1,
            mat_1_to_2,
            pairs,
            curr_box,
            reversed,
        );
    } else if left_hit {
        if leaf1 {
            traverse(
                a,
                b,
                node_a,
                cl2,
                mat_2_to_1,
                mat_1_to_2,
                pairs,
                curr_box,
                reversed,
            );
        } else {
            let new_box = left_box2.apply_matrix4(&mat_2_to_1);
            let cl1 = s1.child_left(n1);
            let cr1 = s1.child_right(n1);
            if new_box.intersects_box(&s1.node_bounds(cl1)) {
                traverse(
                    a,
                    b,
                    if reversed { cl2 } else { cl1 },
                    if reversed { cl1 } else { cl2 },
                    mat_1_to_2,
                    mat_2_to_1,
                    pairs,
                    new_box,
                    !reversed,
                );
            }
            if new_box.intersects_box(&s1.node_bounds(cr1)) {
                traverse(
                    a,
                    b,
                    if reversed { cl2 } else { cr1 },
                    if reversed { cr1 } else { cl2 },
                    mat_1_to_2,
                    mat_2_to_1,
                    pairs,
                    new_box,
                    !reversed,
                );
            }
        }
    } else if right_hit {
        if leaf1 {
            traverse(
                a,
                b,
                node_a,
                cr2,
                mat_2_to_1,
                mat_1_to_2,
                pairs,
                curr_box,
                reversed,
            );
        } else {
            let new_box = right_box2.apply_matrix4(&mat_2_to_1);
            let cl1 = s1.child_left(n1);
            let cr1 = s1.child_right(n1);
            if new_box.intersects_box(&s1.node_bounds(cl1)) {
                traverse(
                    a,
                    b,
                    if reversed { cr2 } else { cl1 },
                    if reversed { cl1 } else { cr2 },
                    mat_1_to_2,
                    mat_2_to_1,
                    pairs,
                    new_box,
                    !reversed,
                );
            }
            if new_box.intersects_box(&s1.node_bounds(cr1)) {
                traverse(
                    a,
                    b,
                    if reversed { cr2 } else { cr1 },
                    if reversed { cr1 } else { cr2 },
                    mat_1_to_2,
                    mat_2_to_1,
                    pairs,
                    new_box,
                    !reversed,
                );
            }
        }
    }
}

/// Upstream MeshBVH leaf iteration: outer B, inner A.
fn collect_leaf_pairs(
    offset_a: usize,
    count_a: usize,
    offset_b: usize,
    count_b: usize,
    pairs: &mut Vec<(usize, usize)>,
) {
    for ib in offset_b..offset_b + count_b {
        for ia in offset_a..offset_a + count_a {
            pairs.push((ia, ib));
        }
    }
}

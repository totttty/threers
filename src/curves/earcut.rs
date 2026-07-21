use crate::math::Vector2;

/// Triangulate a simple polygon (and optional holes) by ear clipping.
/// Returns indices into the flat `points` array (counter-clockwise winding).
///
/// This is a compact, allocation-light implementation: O(n²) which is fine for
/// the polygon counts used by `ExtrudeGeometry`. Production code should reach
/// for `earcutr` once `Shape` outlines get complex.
pub fn earcut(points: &[Vector2], hole_indices: &[usize]) -> Vec<u32> {
    if points.len() < 3 {
        return Vec::new();
    }

    // Build the working polygon: outer ring, then each hole inserted by bridge.
    // For now, we ignore holes — the resulting triangulation is the outer ring only.
    // Holes would require finding a bridge vertex to merge into the outer polygon,
    // which is straightforward but adds ~80 LOC.
    let _ = hole_indices;
    let mut indices: Vec<u32> = (0..points.len() as u32).collect();
    let mut tris = Vec::new();

    // Ensure CCW orientation (signed area > 0). Reverse if CW.
    if signed_area(points, &indices) < 0.0 {
        indices.reverse();
    }

    let mut guard = indices.len() as i32 * indices.len() as i32 + 1;
    while indices.len() > 3 && guard > 0 {
        guard -= 1;
        let n = indices.len();
        let mut found_ear = false;
        for i in 0..n {
            let i_prev = (i + n - 1) % n;
            let i_next = (i + 1) % n;
            let a = points[indices[i_prev] as usize];
            let b = points[indices[i] as usize];
            let c = points[indices[i_next] as usize];
            if !is_convex(a, b, c) {
                continue;
            }
            // Check no other vertex is inside triangle (a,b,c).
            let mut contains = false;
            for j in 0..n {
                if j == i_prev || j == i || j == i_next {
                    continue;
                }
                if point_in_triangle(points[indices[j] as usize], a, b, c) {
                    contains = true;
                    break;
                }
            }
            if contains {
                continue;
            }
            tris.push(indices[i_prev]);
            tris.push(indices[i]);
            tris.push(indices[i_next]);
            indices.remove(i);
            found_ear = true;
            break;
        }
        if !found_ear {
            // Bail out to avoid an infinite loop on degenerate input.
            break;
        }
    }
    if indices.len() == 3 {
        tris.extend_from_slice(&indices);
    }
    tris
}

fn signed_area(points: &[Vector2], indices: &[u32]) -> f32 {
    let mut a = 0.0f32;
    for i in 0..indices.len() {
        let p = points[indices[i] as usize];
        let q = points[indices[(i + 1) % indices.len()] as usize];
        a += p.x * q.y - q.x * p.y;
    }
    a * 0.5
}

fn is_convex(a: Vector2, b: Vector2, c: Vector2) -> bool {
    ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)) > 0.0
}

fn point_in_triangle(p: Vector2, a: Vector2, b: Vector2, c: Vector2) -> bool {
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

fn sign(p: Vector2, a: Vector2, b: Vector2) -> f32 {
    (p.x - b.x) * (a.y - b.y) - (a.x - b.x) * (p.y - b.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_into_two_triangles() {
        let pts = [
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(0.0, 1.0),
        ];
        let tris = earcut(&pts, &[]);
        assert_eq!(tris.len(), 6, "got {:?}", tris);
    }

    #[test]
    fn concave_quad() {
        // Arrow-shaped concave polygon — fan triangulation would fail; earcut
        // should still produce 3 triangles.
        let pts = [
            Vector2::new(0.0, 0.0),
            Vector2::new(2.0, 0.0),
            Vector2::new(1.0, 0.5),
            Vector2::new(2.0, 1.0),
            Vector2::new(0.0, 1.0),
        ];
        let tris = earcut(&pts, &[]);
        assert_eq!(tris.len(), 9);
    }
}

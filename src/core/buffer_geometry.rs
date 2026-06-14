use std::collections::HashMap;
use crate::math::{Box3, Sphere, Vector3};
use super::BufferAttribute;

/// A collection of named vertex attributes plus an optional index buffer.
/// Mirrors three.js's `BufferGeometry`: `geometry.setAttribute("position", ...)`,
/// `geometry.setIndex(...)`.
#[derive(Debug, Clone, Default)]
pub struct BufferGeometry {
    pub attributes: HashMap<String, BufferAttribute>,
    pub index: Option<Vec<u32>>,
    pub bounding_box: Option<Box3>,
    pub bounding_sphere: Option<Sphere>,
    /// Bumped whenever attributes/index change so the renderer re-uploads GPU buffers.
    pub geometry_version: u32,
}

impl BufferGeometry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_attribute(&mut self, name: impl Into<String>, attr: BufferAttribute) -> &mut Self {
        self.attributes.insert(name.into(), attr);
        // Invalidate cached bounds — they depend on positions.
        self.bounding_box = None;
        self.bounding_sphere = None;
        self.geometry_version = self.geometry_version.wrapping_add(1);
        self
    }

    pub fn get_attribute(&self, name: &str) -> Option<&BufferAttribute> {
        self.attributes.get(name)
    }

    pub fn set_index(&mut self, indices: Vec<u32>) -> &mut Self {
        self.index = Some(indices);
        self.geometry_version = self.geometry_version.wrapping_add(1);
        self
    }

    /// Total draw count: index count if indexed, otherwise position count.
    pub fn draw_count(&self) -> usize {
        if let Some(idx) = &self.index {
            idx.len()
        } else {
            self.attributes.get("position").map(|a| a.count()).unwrap_or(0)
        }
    }

    /// Iterate position vertices as `Vector3` (item_size must be 3).
    pub fn positions(&self) -> Option<impl Iterator<Item = Vector3> + '_> {
        let pos = self.attributes.get("position")?;
        if pos.item_size != 3 { return None; }
        Some(pos.array.chunks_exact(3).map(|c| Vector3::new(c[0], c[1], c[2])))
    }

    /// Compute (or refresh) the bounding box from the "position" attribute.
    /// Matches three.js's `computeBoundingBox`.
    pub fn compute_bounding_box(&mut self) -> Box3 {
        let bb = match self.positions() {
            Some(iter) => {
                let mut b = Box3::empty();
                for p in iter { b.expand_by_point(p); }
                if b.is_empty() { Box3::new(Vector3::ZERO, Vector3::ZERO) } else { b }
            }
            None => Box3::new(Vector3::ZERO, Vector3::ZERO),
        };
        self.bounding_box = Some(bb);
        bb
    }

    /// Compute (or refresh) the bounding sphere from positions.
    /// Matches three.js's `computeBoundingSphere`: center on the bounding box
    /// center, then radius = max distance to any vertex.
    pub fn compute_bounding_sphere(&mut self) -> Sphere {
        let bb = self.bounding_box.unwrap_or_else(|| self.compute_bounding_box());
        let center = bb.center();
        let mut max_r2 = 0.0f32;
        if let Some(iter) = self.positions() {
            for p in iter {
                let d2 = (p - center).length_sq();
                if d2 > max_r2 { max_r2 = d2; }
            }
        }
        let s = Sphere::new(center, max_r2.sqrt());
        self.bounding_sphere = Some(s);
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_cube_positions() -> BufferAttribute {
        BufferAttribute::new(
            vec![
                -1.0, -1.0, -1.0,
                 1.0, -1.0, -1.0,
                -1.0,  1.0, -1.0,
                 1.0,  1.0, -1.0,
                -1.0, -1.0,  1.0,
                 1.0, -1.0,  1.0,
                -1.0,  1.0,  1.0,
                 1.0,  1.0,  1.0,
            ],
            3,
        )
    }

    #[test]
    fn bounding_box_of_unit_cube() {
        let mut g = BufferGeometry::new();
        g.set_attribute("position", unit_cube_positions());
        let b = g.compute_bounding_box();
        assert_eq!(b.min, Vector3::new(-1.0, -1.0, -1.0));
        assert_eq!(b.max, Vector3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn bounding_sphere_of_unit_cube_radius() {
        let mut g = BufferGeometry::new();
        g.set_attribute("position", unit_cube_positions());
        let s = g.compute_bounding_sphere();
        assert_eq!(s.center, Vector3::ZERO);
        assert!((s.radius - 3.0f32.sqrt()).abs() < 1e-5);
    }
}

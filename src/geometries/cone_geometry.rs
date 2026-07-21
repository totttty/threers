use super::CylinderGeometry;
use crate::core::BufferGeometry;
use std::f32::consts::PI;

pub struct ConeGeometry;

impl ConeGeometry {
    /// Cone is a cylinder with `radius_top = 0`. Matches three.js.
    pub fn new(
        radius: f32,
        height: f32,
        radial_segments: usize,
        height_segments: usize,
        open_ended: bool,
        theta_start: f32,
        theta_length: f32,
    ) -> BufferGeometry {
        CylinderGeometry::new(
            0.0,
            radius,
            height,
            radial_segments,
            height_segments,
            open_ended,
            theta_start,
            theta_length,
        )
    }

    pub fn default_(radius: f32, height: f32) -> BufferGeometry {
        Self::new(radius, height, 32, 1, false, 0.0, PI * 2.0)
    }
}

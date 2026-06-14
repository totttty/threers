//! Built-in geometry generators, mirroring three.js's geometry module.

mod box_geometry;
mod plane_geometry;
mod sphere_geometry;
mod circle_geometry;
mod ring_geometry;
mod cylinder_geometry;
mod cone_geometry;
mod torus_geometry;
mod torus_knot_geometry;
mod capsule_geometry;
mod polyhedron_geometry;
mod edges_geometry;
mod lathe_geometry;
mod tube_geometry;
mod extrude_geometry;
mod parametric_geometry;
mod convex_geometry;
mod decal_geometry;
mod text_geometry;
mod box_line;

pub use box_geometry::BoxGeometry;
pub use plane_geometry::PlaneGeometry;
pub use sphere_geometry::SphereGeometry;
pub use circle_geometry::CircleGeometry;
pub use ring_geometry::RingGeometry;
pub use cylinder_geometry::CylinderGeometry;
pub use cone_geometry::ConeGeometry;
pub use torus_geometry::TorusGeometry;
pub use torus_knot_geometry::TorusKnotGeometry;
pub use capsule_geometry::CapsuleGeometry;
pub use polyhedron_geometry::{
    PolyhedronGeometry, TetrahedronGeometry, OctahedronGeometry,
    IcosahedronGeometry, DodecahedronGeometry,
};
pub use edges_geometry::{EdgesGeometry, WireframeGeometry};
pub use lathe_geometry::LatheGeometry;
pub use tube_geometry::TubeGeometry;
pub use extrude_geometry::ExtrudeGeometry;
pub use parametric_geometry::ParametricGeometry;
pub use convex_geometry::ConvexGeometry;
pub use decal_geometry::DecalGeometry;
pub use text_geometry::{TextGeometry, Glyph};
pub use box_line::BoxLineGeometry;

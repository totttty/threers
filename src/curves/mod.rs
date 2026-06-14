//! Parametric curves. Mirror three.js's curve hierarchy: `Curve` base +
//! 2D and 3D concretes + `CurvePath`/`Path`/`Shape`.

mod curve;
mod line_curve;
mod bezier;
mod ellipse;
mod catmull_rom;
mod spline;
mod curve_path;
mod path;
mod shape;
mod nurbs;
pub mod earcut;

pub use curve::{Curve2, Curve3};
pub use line_curve::{LineCurve, LineCurve3};
pub use bezier::{QuadraticBezierCurve, QuadraticBezierCurve3, CubicBezierCurve, CubicBezierCurve3};
pub use ellipse::EllipseCurve;
pub use catmull_rom::CatmullRomCurve3;
pub use spline::SplineCurve;
pub use curve_path::CurvePath;
pub use path::Path;
pub use shape::Shape;
pub use nurbs::{NURBSCurve, NURBSSurface};

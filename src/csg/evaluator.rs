//! Boolean evaluator — mirrors `web/csg/core/Evaluator.js`.

use crate::core::BufferGeometry;

use super::attribute_data::TypedAttributeData;
use super::brush::CsgBrush;
use super::constants::Operation;
use super::operations::{assign_buffer_data, perform_operation};
use super::triangle_splitter::TriangleSplitter;

/// Evaluates boolean operations between [`CsgBrush`]es.
///
/// Set [`Self::use_groups`] to preserve multi-material group indices (parity scenes
/// typically leave this `false`).
#[derive(Debug, Default)]
pub struct CsgEvaluator {
    pub use_groups: bool,
    splitter: TriangleSplitter,
    attribute_data: Vec<TypedAttributeData>,
}

impl CsgEvaluator {
    pub fn new() -> Self {
        Self {
            use_groups: false,
            splitter: TriangleSplitter::new(),
            attribute_data: vec![TypedAttributeData::new()],
        }
    }

    /// Evaluate a single boolean op; returns a non-indexed triangle soup.
    pub fn evaluate(
        &mut self,
        a: &mut CsgBrush,
        b: &mut CsgBrush,
        operation: Operation,
    ) -> BufferGeometry {
        self.evaluate_ops(a, b, &[operation])
    }

    /// Evaluate one or more ops in a single pass (batch / hollow variants).
    pub fn evaluate_ops(
        &mut self,
        a: &mut CsgBrush,
        b: &mut CsgBrush,
        operations: &[Operation],
    ) -> BufferGeometry {
        let primary_op = operations[0];
        let _ = primary_op;
        while self.attribute_data.len() < operations.len() {
            self.attribute_data.push(TypedAttributeData::new());
        }
        for data in &mut self.attribute_data[..operations.len()] {
            data.clear();
            data.initialize_array("position", 3);
            data.initialize_array("normal", 3);
        }
        perform_operation(
            a,
            b,
            operations,
            &mut self.splitter,
            &mut self.attribute_data[..operations.len()],
            self.use_groups,
        );
        let mut out = BufferGeometry::new();
        assign_buffer_data(&mut out, &self.attribute_data[0]);
        out
    }

    pub fn reset(&mut self) {
        self.splitter.reset();
        for d in &mut self.attribute_data {
            d.clear();
        }
    }
}

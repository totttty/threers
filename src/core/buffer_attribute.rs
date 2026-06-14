/// A typed vertex attribute, equivalent to three.js's `BufferAttribute`.
///
/// `item_size` is the number of components per vertex (e.g. 3 for position).
#[derive(Debug, Clone)]
pub struct BufferAttribute {
    pub array: Vec<f32>,
    pub item_size: usize,
}

impl BufferAttribute {
    pub fn new(array: Vec<f32>, item_size: usize) -> Self {
        Self { array, item_size }
    }

    pub fn count(&self) -> usize {
        self.array.len() / self.item_size
    }
}

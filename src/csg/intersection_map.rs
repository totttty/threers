use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct IntersectionMap {
    pub intersection_set: HashMap<usize, Vec<usize>>,
    pub ids: Vec<usize>,
}

impl IntersectionMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, id: usize, intersection_id: usize) {
        if !self.intersection_set.contains_key(&id) {
            self.intersection_set.insert(id, Vec::new());
            self.ids.push(id);
        }
        self.intersection_set.get_mut(&id).unwrap().push(intersection_id);
    }
}

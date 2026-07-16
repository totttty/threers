use super::mesh_bvh::MeshBvh;

pub const SERIALIZE_VERSION: u32 = 1;

/// Plain-object representation for transfer across workers / storage.
#[derive(Debug, Clone)]
pub struct SerializedMeshBvh {
    pub version: u32,
    pub node_buffer: Vec<f32>,
    pub triangle_order: Vec<u32>,
    pub triangle_indices: Vec<u32>,
    pub positions: Vec<f32>,
}

impl SerializedMeshBvh {
    pub fn from_bvh(bvh: &MeshBvh) -> Self {
        Self {
            version: SERIALIZE_VERSION,
            node_buffer: bvh.node_buffer().to_vec(),
            triangle_order: bvh.triangle_order().iter().map(|&i| i as u32).collect(),
            triangle_indices: bvh
                .triangle_indices()
                .iter()
                .flat_map(|(a, b, c)| [*a, *b, *c])
                .collect(),
            positions: bvh.positions().to_vec(),
        }
    }

    pub fn into_bvh(self) -> Option<MeshBvh> {
        MeshBvh::deserialize(self)
    }
}

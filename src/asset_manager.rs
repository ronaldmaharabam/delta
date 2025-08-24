use std::{collections::HashMap, sync::Arc};

pub mod importer;
pub mod mesh;

use importer::Importer;
use slotmap::{SlotMap, new_key_type};

new_key_type! {
    pub struct MeshId;
    pub struct MaterialId;
    pub struct TextureId;
}

pub struct AssetManager<I: Importer> {
    pub importer: I,

    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,

    pub meshes_by_name: HashMap<String, MeshId>,

    pub meshes: SlotMap<MeshId, mesh::Mesh>,
}

impl<I: Importer> AssetManager<I> {
    pub fn new(importer: I, device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            importer,
            device,
            queue,
            meshes_by_name: HashMap::new(),
            meshes: SlotMap::with_key(),
        }
    }
}


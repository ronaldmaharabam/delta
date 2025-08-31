use std::{collections::HashMap, sync::Arc};

pub mod importer;
pub mod light;
pub mod material;
pub mod mesh;

use importer::GltfImporter;
use material::{MAX_MAT, MaterialUniform};
use slotmap::{SlotMap, new_key_type};

new_key_type! {
    pub struct MeshId;
    pub struct MaterialId;
    pub struct TextureId;
}

pub struct AssetManager {
    pub importer: GltfImporter,

    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,

    pub meshes_by_name: HashMap<String, MeshId>,

    pub meshes: SlotMap<MeshId, mesh::Mesh>,

    pub mat_buffer: wgpu::Buffer,

    pub mat_free: Vec<usize>,
}

impl AssetManager {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let mat_buffer_size =
            (MAX_MAT * std::mem::size_of::<MaterialUniform>()) as wgpu::BufferAddress;

        let mat_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Material Buffer"),
            size: mat_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            importer: GltfImporter::new(),
            device,
            queue,
            meshes_by_name: HashMap::new(),
            meshes: SlotMap::with_key(),
            mat_buffer: mat_buffer,
            mat_free: (0..MAX_MAT).collect(),
        }
    }
}

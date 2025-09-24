use std::{collections::HashMap, sync::Arc};

pub mod importer;
pub mod light;
pub mod material;
pub mod mesh;
pub mod texture;

use importer::GltfImporter;
use material::{MAX_MAT, MaterialUniform};
use slotmap::{SlotMap, new_key_type};

use crate::asset_manager::{
    material::MaterialId,
    texture::{GpuTexture, TextureGroup, TextureKey},
};

new_key_type! {
    pub struct MeshId;
    pub struct TextureId;
    pub struct SamplerId;
}

pub struct AssetManager {
    pub importer: GltfImporter,

    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,

    pub meshes_by_name: HashMap<String, MeshId>,

    pub meshes: SlotMap<MeshId, mesh::Mesh>,

    pub mat_buffer: wgpu::Buffer,
    pub mat_free: Vec<usize>,
    pub mat_by_name: HashMap<String, MaterialId>,
    pub tex_by_mat: Vec<TextureGroup>,

    pub tex_by_key: HashMap<TextureKey, TextureId>,
    pub textures: SlotMap<TextureId, GpuTexture>,

    pub sampler_default: SamplerId,
    pub sampler_by_name: HashMap<String, SamplerId>,
    pub samplers: SlotMap<SamplerId, wgpu::Sampler>,
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

        let default_uniform = MaterialUniform::default();
        queue.write_buffer(&mat_buffer, 0, bytemuck::bytes_of(&default_uniform));

        let mut samplers = SlotMap::with_key();

        let sampler_default: SamplerId =
            samplers.insert(device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Default Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }));

        Self {
            importer: GltfImporter::new(),
            device,
            queue,
            meshes_by_name: HashMap::new(),
            meshes: SlotMap::with_key(),
            mat_buffer,
            mat_free: (1..MAX_MAT).rev().collect(),
            mat_by_name: HashMap::new(),
            tex_by_key: HashMap::new(),
            textures: SlotMap::with_key(),
            tex_by_mat: Vec::with_capacity(MAX_MAT as usize),
            sampler_by_name: HashMap::new(),
            samplers,
            sampler_default,
        }
    }
    fn split_key<'a>(key: &'a str) -> (&'a str, Option<&'a str>) {
        let mut it = key.splitn(2, '#');
        let path = it.next().unwrap_or(key);
        let selector = it.next();
        (path, selector)
    }
    fn split_path<'a>(key: &'a str) -> Result<(&'a str, usize), ()> {
        let mut it = key.splitn(2, '#');
        let path = it.next().unwrap();

        let selector_str = it.next().ok_or(())?;

        let selector = selector_str.parse::<usize>().map_err(|_| ())?;

        Ok((path, selector))
    }
}
#[derive(Debug)]
pub enum SplitPathError {
    MissingSeparator,
    InvalidSelector,
}

use std::{collections::HashMap, num::NonZeroU32, sync::Arc};

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
    texture::{MAX_COLOR_TEXTURES, MAX_DATA_TEXTURES, MAX_DEPTH_TEXTURES},
};

new_key_type! {
    pub struct MeshId;
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

    // Bindless texture arrays
    pub color_textures: Vec<wgpu::TextureView>, // sRGB: baseColor, emissive
    pub data_textures: Vec<wgpu::TextureView>,  // linear: normal, MR, AO
    pub depth_textures: Vec<wgpu::TextureView>, // shadow maps

    pub color_samplers: Vec<wgpu::Sampler>,
    pub data_samplers: Vec<wgpu::Sampler>,
    pub depth_samplers: Vec<wgpu::Sampler>,

    // Global bind group
    pub texture_bind_group: wgpu::BindGroup,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
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

        let dummy_white = Self::create_color_texture(&device, &queue, &[255, 255, 255, 255], 1, 1);
        let dummy_normal = Self::create_data_texture(&device, &queue, &[128, 128, 255, 255], 1, 1);
        let dummy_depth = Self::create_depth_texture(&device, 1, 1);

        let color_textures = vec![dummy_white.create_view(&wgpu::TextureViewDescriptor::default())];
        let data_textures = vec![dummy_normal.create_view(&wgpu::TextureViewDescriptor::default())];
        let depth_textures = vec![dummy_depth.create_view(&wgpu::TextureViewDescriptor::default())];

        let color_samplers = vec![device.create_sampler(&wgpu::SamplerDescriptor::default())];
        let data_samplers = vec![device.create_sampler(&wgpu::SamplerDescriptor::default())];
        let depth_samplers = vec![device.create_sampler(&wgpu::SamplerDescriptor {
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        })];

        // --- Layout ---
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // color
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: NonZeroU32::new(MAX_COLOR_TEXTURES),
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: NonZeroU32::new(MAX_COLOR_TEXTURES),
                    },
                    // data
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: NonZeroU32::new(MAX_DATA_TEXTURES),
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: NonZeroU32::new(MAX_DATA_TEXTURES),
                    },
                    // depth
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: NonZeroU32::new(MAX_DEPTH_TEXTURES),
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: NonZeroU32::new(MAX_DEPTH_TEXTURES),
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[],
            label: Some("texture_bind_group"),
        });

        Self {
            importer: GltfImporter::new(),
            device,
            queue,
            meshes_by_name: HashMap::new(),
            meshes: SlotMap::with_key(),
            mat_buffer,
            mat_free: (1..MAX_MAT).rev().collect(),
            mat_by_name: HashMap::new(),
            color_textures,
            data_textures,
            depth_textures,
            color_samplers,
            data_samplers,
            depth_samplers,
            texture_bind_group,
            texture_bind_group_layout,
        }
    }
    fn split_key<'a>(key: &'a str) -> (&'a str, Option<&'a str>) {
        let mut it = key.splitn(2, '#');
        let path = it.next().unwrap_or(key);
        let selector = it.next();
        (path, selector)
    }
}

use super::{AssetManager, texture::TextureId};

pub const MAX_MAT: usize = 1024;

#[derive(Debug, Clone, Copy)]
pub struct MaterialId(pub usize);
impl From<usize> for MaterialId {
    fn from(v: usize) -> Self {
        MaterialId(v)
    }
}

impl From<MaterialId> for usize {
    fn from(id: MaterialId) -> Self {
        id.0
    }
}

#[derive(Debug, Clone)]
pub struct Material {
    pub base_color_factor: [f32; 4],
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub emissive_factor: [f32; 3],
    pub alpha_cutoff: f32,
    pub double_sided: bool,
    pub base_color_texture: Option<usize>,
    pub metallic_roughness_texture: Option<usize>,
    pub normal_texture: Option<usize>,
    pub emissive_texture: Option<usize>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
    pub base_color_factor: [f32; 4],
    pub emissive_factor: [f32; 3],
    pub emissive_padding: f32,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub alpha_cutoff: f32,
    pub double_sided: u32,
    pub texture_indices: [i32; 4],
}
impl Default for MaterialUniform {
    fn default() -> Self {
        Self {
            base_color_factor: [0.0, 1.0, 0.5, 1.0],
            emissive_factor: [0.0, 5.0, 2.5],
            emissive_padding: 0.0,
            metallic_factor: -1.0,
            roughness_factor: -1.0,
            alpha_cutoff: -1.0,
            double_sided: 12345,
            texture_indices: [-999; 4],
        }
    }
}

impl From<&Material> for MaterialUniform {
    fn from(m: &Material) -> Self {
        Self {
            base_color_factor: m.base_color_factor,
            emissive_factor: m.emissive_factor,
            emissive_padding: 0.0,
            metallic_factor: m.metallic_factor,
            roughness_factor: m.roughness_factor,
            alpha_cutoff: m.alpha_cutoff,
            double_sided: if m.double_sided { 1 } else { 0 },
            texture_indices: [0; 4],
        }
    }
}
impl AssetManager {
    pub fn get_material(&mut self, name: &str) -> MaterialId {
        if let Some(&id) = self.mat_by_name.get(name) {
            return id;
        }

        let (path, selector) = Self::split_key(name);

        let material = self.importer.load_material(path, selector);

        let base_color_tex = material
            .base_color_texture
            .map(|i| self.get_texture(&format!("{}#{}", path, i)))
            .unwrap_or(0.into());

        let metallic_roughness_tex = material
            .metallic_roughness_texture
            .map(|i| self.get_texture(&format!("{}#{}", path, i)))
            .unwrap_or(0.into());

        let normal_tex = material
            .normal_texture
            .map(|i| self.get_texture(&format!("{}#{}", path, i)))
            .unwrap_or(0.into());

        let emissive_tex = material
            .emissive_texture
            .map(|i| self.get_texture(&format!("{}#{}", path, i)))
            .unwrap_or(0.into());

        let uniform: MaterialUniform = MaterialUniform {
            base_color_factor: material.base_color_factor,
            metallic_factor: material.metallic_factor,
            roughness_factor: material.roughness_factor,
            emissive_factor: material.emissive_factor,
            emissive_padding: 0.0,
            alpha_cutoff: material.alpha_cutoff,
            double_sided: material.double_sided as u32,
            texture_indices: [
                base_color_tex.0 as i32,
                metallic_roughness_tex.0 as i32,
                normal_tex.0 as i32,
                emissive_tex.0 as i32,
            ],
        };

        let idx = self
            .mat_free
            .pop()
            .expect("No free material slots available");

        let offset = (idx * std::mem::size_of::<MaterialUniform>()) as wgpu::BufferAddress;
        self.queue
            .write_buffer(&self.mat_buffer, offset, bytemuck::bytes_of(&uniform));

        self.mat_by_name.insert(name.to_string(), idx.into());
        idx.into()
    }
}

#[repr(C, align(16))]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MatId {
    pub id: u32,
    pub _pad: [u32; 63],
}

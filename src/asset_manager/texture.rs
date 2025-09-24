use crate::asset_manager::AssetManager;
use crate::asset_manager::SamplerId;
use crate::asset_manager::TextureId;
use wgpu::util::DeviceExt; // to get the trait with create_texture_with_data
use wgpu::util::TextureDataOrder;
pub const MAX_COLOR_TEXTURES: u32 = 1024;
pub const MAX_DATA_TEXTURES: u32 = 1024;
pub const MAX_DEPTH_TEXTURES: u32 = 1024;
//#[derive(Debug, Clone, Copy)]
//pub struct TextureId(pub usize);
//
//impl From<usize> for TextureId {
//    fn from(v: usize) -> Self {
//        TextureId(v)
//    }
//}
//
//impl From<TextureId> for usize {
//    fn from(id: TextureId) -> Self {
//        id.0
//    }
//}
//

#[derive(Debug, Clone)]
pub enum AddressMode {
    ClampToEdge,
    MirrorRepeat,
    Repeat,
}

#[derive(Debug, Clone)]
pub enum FilterMode {
    Nearest,
    Linear,
}

#[derive(Debug, Clone)]
pub struct Sampler {
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
    pub mag_filter: FilterMode,
    pub min_filter: FilterMode,
    pub mipmap_filter: FilterMode,
}
pub struct Texture {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub sampler: Option<usize>,
}
#[derive(Hash, Eq, PartialEq, Clone)]
pub struct TextureKey {
    key: String,
    format: wgpu::TextureFormat,
}

pub struct GpuTexture {
    pub tex: wgpu::Texture,
    pub tex_view: wgpu::TextureView,
    pub sampler: SamplerId,
}

#[derive(Debug, Clone, Copy)]
pub struct TextureGroup {
    pub base_color: TextureId,
    pub metallic_roughness: TextureId,
    pub normal: TextureId,
    pub emissive: TextureId,
    pub occlusion: TextureId,
}
impl AssetManager {
    pub fn get_texture(&mut self, key: &str, format: wgpu::TextureFormat) -> TextureId {
        let tex_key = TextureKey {
            key: key.to_string(),
            format,
        };

        if let Some(&id) = self.tex_by_key.get(&tex_key) {
            return id;
        }

        let (path, selector) =
            Self::split_path(key).expect("get_texture: key not valid! expected in form path#0");

        let tex_data = self.importer.load_texture(path, selector);

        let sampler_id = if let Some(sampler_index) = tex_data.sampler {
            let sampler_key = format!("{}#{}", path, sampler_index);
            self.get_sampler(&sampler_key)
        } else {
            self.sampler_default
        };

        let texture = self.device.create_texture_with_data(
            &self.queue,
            &wgpu::TextureDescriptor {
                label: Some(key),
                size: wgpu::Extent3d {
                    width: tex_data.width,
                    height: tex_data.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &tex_data.pixels,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let new_id = self.textures.insert(GpuTexture {
            tex: texture,
            tex_view: view,
            sampler: sampler_id,
        });

        self.tex_by_key.insert(tex_key, new_id);
        new_id
    }

    pub fn get_sampler(&mut self, key: &str) -> SamplerId {
        let (path, selector) = Self::split_path(key).expect(&format!(
            "get_sampler: {} is not a valid key! Expected format is path#0",
            key
        ));

        *self
            .sampler_by_name
            .entry(key.to_string())
            .or_insert_with(|| {
                let sampler_info = self.importer.load_sampler(path, selector);

                let wrap = |m: AddressMode| match m {
                    AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
                    AddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
                    AddressMode::Repeat => wgpu::AddressMode::Repeat,
                };

                let filter = |f: FilterMode| match f {
                    FilterMode::Nearest => wgpu::FilterMode::Nearest,
                    FilterMode::Linear => wgpu::FilterMode::Linear,
                };

                let new_sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some(key),
                    address_mode_u: wrap(sampler_info.address_mode_u),
                    address_mode_v: wrap(sampler_info.address_mode_v),
                    address_mode_w: wrap(sampler_info.address_mode_w),

                    mag_filter: filter(sampler_info.mag_filter),
                    min_filter: filter(sampler_info.min_filter),
                    mipmap_filter: filter(sampler_info.mipmap_filter),
                    ..Default::default()
                });
                self.samplers.insert(new_sampler)
            })
    }

    pub fn create_color_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> wgpu::Texture {
        device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("color_texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            data,
        )
    }

    pub fn create_data_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> wgpu::Texture {
        device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("data_texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            data,
        )
    }
    pub fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }
}

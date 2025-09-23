use crate::asset_manager::AssetManager;
use wgpu::util::DeviceExt; // to get the trait with create_texture_with_data
use wgpu::util::TextureDataOrder;
pub const MAX_COLOR_TEXTURES: u32 = 1024;
pub const MAX_DATA_TEXTURES: u32 = 1024;
pub const MAX_DEPTH_TEXTURES: u32 = 1024;
#[derive(Debug, Clone, Copy)]
pub struct TextureId(pub usize);

impl From<usize> for TextureId {
    fn from(v: usize) -> Self {
        TextureId(v)
    }
}

impl From<TextureId> for usize {
    fn from(id: TextureId) -> Self {
        id.0
    }
}

impl AssetManager {
    pub fn get_texture(&mut self, _name: &str) -> TextureId {
        0.into()
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

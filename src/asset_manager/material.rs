use crate::asset_manager::TextureId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaMode {
    Opaque,
    Mask,
    Blend,
}

impl Default for AlphaMode {
    fn default() -> Self {
        AlphaMode::Opaque
    }
}

#[derive(Clone, Debug)]
pub struct TextureRef {
    pub tex: TextureId,
    pub texcoord: u32,
}

#[derive(Clone, Debug)]
pub struct Material {
    pub name: Option<String>,

    pub base_color_factor: [f32; 4],
    //pub base_color_texture: Option<TextureRef>,
    pub metallic_factor: f32,
    pub roughness_factor: f32,

    //pub metallic_roughness_texture: Option<TextureRef>,
    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: f32,
    pub double_sided: bool,
}

impl Material {}
pub struct MaterialUniform {}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: None,
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            //base_color_texture: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            //metallic_roughness_texture: None,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
            double_sided: false,
        }
    }
}

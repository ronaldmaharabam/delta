use crate::asset_manager::AssetManager;

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
}

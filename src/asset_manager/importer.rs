use super::{
    material::Material,
    mesh::{Index, Primitive, Vertex},
};
use crate::asset_manager::texture::{AddressMode, FilterMode, Sampler, Texture};
use gltf::{self, Gltf, import, mesh::Mode};

pub struct GltfImporter;

impl GltfImporter {
    pub fn new() -> Self {
        Self
    }

    //fn split_key<'a>(key: &'a str) -> (&'a str, Option<&'a str>) {
    //    let mut it = key.splitn(2, '#');
    //    let path = it.next().unwrap_or(key);
    //    let selector = it.next();
    //    (path, selector)
    //}

    fn select_mesh<'a>(doc: &'a gltf::Document, sel: Option<&str>, path: &str) -> gltf::Mesh<'a> {
        if let Some(s) = sel {
            if let Ok(idx) = s.parse::<usize>() {
                doc.meshes()
                    .nth(idx)
                    .unwrap_or_else(|| panic!("Mesh index {idx} not found in '{path}'"))
            } else {
                doc.meshes()
                    .find(|m| m.name().map(|n| n == s).unwrap_or(false))
                    .unwrap_or_else(|| panic!("Mesh named '{s}' not found in '{path}'"))
            }
        } else {
            doc.meshes().next().expect("No meshes in glTF file")
        }
    }

    fn select_material<'a>(
        doc: &'a gltf::Document,
        sel: Option<&str>,
        path: &str,
    ) -> gltf::Material<'a> {
        if let Some(s) = sel {
            if let Ok(idx) = s.parse::<usize>() {
                doc.materials()
                    .nth(idx)
                    .unwrap_or_else(|| panic!("Material index {idx} not found in '{path}'"))
            } else {
                doc.materials()
                    .find(|m| m.name().map(|n| n == s).unwrap_or(false))
                    .unwrap_or_else(|| panic!("Material named '{s}' not found in '{path}'"))
            }
        } else {
            doc.materials().next().expect("No materials in glTF file")
        }
    }

    pub fn load_mesh(&mut self, path: &str, selector: Option<&str>) -> Vec<Primitive> {
        let (doc, buffers, _images) = gltf::import(path).expect("Failed to load glTF file");
        let mesh = Self::select_mesh(&doc, selector, path);

        let mut out: Vec<Primitive> = Vec::new();

        for prim in mesh.primitives() {
            if prim.mode() != Mode::Triangles {
                panic!("Unsupported primitive mode: {:?}", prim.mode());
            }

            let reader = prim.reader(|buffer| Some(&buffers[buffer.index()].0[..]));

            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .expect("Primitive missing POSITION attribute")
                .collect();

            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|it| it.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|tc| tc.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

            let tangents: Vec<[f32; 4]> = reader
                .read_tangents()
                .map(|it| it.collect())
                .unwrap_or_else(|| vec![[1.0, 0.0, 0.0, 1.0]; positions.len()]);

            let indices: Vec<u32> = match reader.read_indices() {
                Some(gltf::mesh::util::ReadIndices::U8(i)) => i.map(|v| v as u32).collect(),
                Some(gltf::mesh::util::ReadIndices::U16(i)) => i.map(|v| v as u32).collect(),
                Some(gltf::mesh::util::ReadIndices::U32(i)) => i.collect(),
                None => (0u32..positions.len() as u32).collect(),
            };

            let vertices = (0..positions.len())
                .map(|i| Vertex {
                    position: positions[i],
                    uv: uvs[i],
                    normal: normals[i],
                    tangent: tangents[i],
                })
                .collect::<Vec<_>>();

            let tri_indices = indices
                .chunks(3)
                .filter(|tri| tri.len() == 3)
                .map(|tri| Index {
                    idx: [tri[0], tri[1], tri[2]],
                })
                .collect::<Vec<_>>();

            let material = prim.material().index();
            out.push(Primitive {
                vertex: vertices,
                index: tri_indices,
                material,
            });
        }

        out
    }

    pub fn load_material(&mut self, path: &str, selector: Option<&str>) -> Material {
        let (doc, _buffers, _images) = gltf::import(path).expect("Failed to load glTF file");
        let material = Self::select_material(&doc, selector, path);

        let pbr = material.pbr_metallic_roughness();

        Material {
            base_color_factor: pbr.base_color_factor(),
            metallic_factor: pbr.metallic_factor(),
            roughness_factor: pbr.roughness_factor(),
            emissive_factor: material.emissive_factor(),
            alpha_cutoff: material.alpha_cutoff().unwrap_or(0.5),
            double_sided: material.double_sided(),
            base_color_texture: pbr.base_color_texture().map(|info| info.texture().index()),
            metallic_roughness_texture: pbr
                .metallic_roughness_texture()
                .map(|info| info.texture().index()),
            normal_texture: material.normal_texture().map(|info| info.texture().index()),
            emissive_texture: material
                .emissive_texture()
                .map(|info| info.texture().index()),
        }
    }

    pub fn load_texture(&mut self, path: &str, selector: usize) -> Texture {
        let (doc, buffers, _images) = import(path).expect("Failed to import glTF");
        let tex = doc.textures().nth(selector).expect("Invalid texture index");
        let img = tex.source().source();
        let sampler_index = tex.sampler().index();

        let (pixels, width, height) = match img {
            gltf::image::Source::View { view, mime_type: _ } => {
                let buffer = &buffers[view.buffer().index()];
                let start = view.offset();
                let end = start + view.length();
                let data = &buffer[start..end];
                let dyn_img =
                    image::load_from_memory(data).expect("Failed to decode embedded image");
                let rgba = dyn_img.to_rgba8();
                let (w, h) = rgba.dimensions();
                // Get the raw Vec<u8> directly
                let pixels = rgba.into_raw();
                (pixels, w, h)
            }
            gltf::image::Source::Uri { uri, mime_type: _ } => {
                let parent = std::path::Path::new(path)
                    .parent()
                    .unwrap_or(std::path::Path::new("."));
                let img_path = parent.join(uri);
                let dyn_img = image::open(img_path).expect("Failed to open external image");
                let rgba = dyn_img.to_rgba8();
                let (w, h) = rgba.dimensions();
                // Get the raw Vec<u8> directly
                let pixels = rgba.into_raw();
                (pixels, w, h)
            }
        };

        Texture {
            pixels, // This is now a Vec<u8>
            width,
            height,
            sampler: sampler_index,
        }
    }
    pub fn load_sampler(&mut self, path: &str, selector: usize) -> Sampler {
        let gltf = Gltf::open(path).expect("Failed to open glTF file");
        let s = gltf
            .samplers()
            .nth(selector)
            .expect("Sampler index out of range");

        let wrap = |mode: gltf::texture::WrappingMode| match mode {
            gltf::texture::WrappingMode::ClampToEdge => AddressMode::ClampToEdge,
            gltf::texture::WrappingMode::MirroredRepeat => AddressMode::MirrorRepeat,
            gltf::texture::WrappingMode::Repeat => AddressMode::Repeat,
        };

        let mag = match s.mag_filter() {
            Some(gltf::texture::MagFilter::Nearest) => FilterMode::Nearest,
            Some(gltf::texture::MagFilter::Linear) | None => FilterMode::Linear,
        };

        let (min, mipmap) = match s.min_filter() {
            Some(gltf::texture::MinFilter::Nearest) => (FilterMode::Nearest, FilterMode::Nearest),
            Some(gltf::texture::MinFilter::Linear) => (FilterMode::Linear, FilterMode::Nearest),
            Some(gltf::texture::MinFilter::NearestMipmapNearest) => {
                (FilterMode::Nearest, FilterMode::Nearest)
            }
            Some(gltf::texture::MinFilter::LinearMipmapNearest) => {
                (FilterMode::Linear, FilterMode::Nearest)
            }
            Some(gltf::texture::MinFilter::NearestMipmapLinear) => {
                (FilterMode::Nearest, FilterMode::Linear)
            }
            Some(gltf::texture::MinFilter::LinearMipmapLinear) => {
                (FilterMode::Linear, FilterMode::Linear)
            }
            None => (FilterMode::Linear, FilterMode::Nearest),
        };

        let sampler = Sampler {
            address_mode_u: wrap(s.wrap_s()),
            address_mode_v: wrap(s.wrap_t()),
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: mag,
            min_filter: min,
            mipmap_filter: mipmap,
        };

        sampler
    }
}

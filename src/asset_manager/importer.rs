use super::mesh::{Index, Primitive, Vertex};
use crate::asset_manager::{MaterialId, TextureId};
use gltf::{self, mesh::Mode};

//pub trait Importer {
//    fn load_mesh(&mut self, key: &str) -> Vec<Primitive>;
//    //fn load_material(&mut self, key: &str) -> MaterialId;
//    //fn load_textture(&mut self, key: &str) -> TextureId;
//
//    fn new() -> Self;
//}

pub struct GltfImporter {}

impl GltfImporter {
    pub fn new() -> Self {
        Self {}
    }
    pub fn load_mesh(&mut self, key: &str) -> Vec<Primitive> {
        let (path, mesh_name_opt) = {
            let mut it = key.splitn(2, '#');
            let path = it.next().unwrap_or(key);
            let mesh_name_opt = it.next();
            (path, mesh_name_opt)
        };

        println!("{}", path);
        let (doc, buffers, _images) = gltf::import(path).expect("Failed to load glTF file");

        let mesh = if let Some(target_name) = mesh_name_opt {
            doc.meshes()
                .find(|m| m.name().map(|n| n == target_name).unwrap_or(false))
                .unwrap_or_else(|| panic!("Mesh named '{target_name}' not found in '{path}'"))
        } else {
            doc.meshes().next().expect("No meshes in glTF file")
        };

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

            let uvs_f32: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|tc| tc.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

            let indices_u32: Vec<u32> = match reader.read_indices() {
                Some(gltf::mesh::util::ReadIndices::U8(i)) => i.map(|v| v as u32).collect(),
                Some(gltf::mesh::util::ReadIndices::U16(i)) => i.map(|v| v as u32).collect(),
                Some(gltf::mesh::util::ReadIndices::U32(i)) => i.collect(),
                None => (0u32..positions.len() as u32).collect(),
            };

            let n = positions.len().min(normals.len()).min(uvs_f32.len());
            let mut vertices = Vec::with_capacity(n);
            for i in 0..n {
                vertices.push(Vertex {
                    position: positions[i],
                    normal: normals[i],
                    uv: uvs_f32[i],
                });
            }

            let mut tri_indices = Vec::with_capacity(indices_u32.len() / 3);
            for tri in indices_u32.chunks(3) {
                if tri.len() < 3 {
                    break;
                }
                tri_indices.push(Index {
                    idx: [tri[0], tri[1], tri[2]],
                });
            }

            let material: MaterialId = Default::default();

            out.push(Primitive {
                vertex: vertices,
                index: tri_indices,
                material,
            });
        }

        out
    }
}

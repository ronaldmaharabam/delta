use super::{AssetManager, MaterialId, MeshId, importer::Importer};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

// ---- fix: typo + make Vertex/Index Pod so we can cast to &[u8] ----
#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
struct Index {
    idx: [u32; 3],
}

struct Primitive {
    vertex: Vec<Vertex>,
    index: Vec<Index>,
    default_material: MaterialId,
}

pub struct Mesh {
    pub name: Option<String>,
    pub primitives: Vec<Primitive>,
    pub vertex_buf: wgpu::Buffer,
    pub index_buf: Option<wgpu::Buffer>,
}

impl<I: Importer> AssetManager<I> {
    pub fn get_mesh(&mut self, name: &str) -> MeshId {
        if let Some(&id) = self.meshes_by_name.get(name) {
            return id;
        }

        let primitives = self.importer.load_mesh(name);

        let mut flat_vertices: Vec<Vertex> = Vec::new();
        let mut flat_indices_u32: Vec<u32> = Vec::new();

        let mut base_vertex: u32 = 0;
        for prim in &primitives {
            flat_vertices.extend_from_slice(&prim.vertex);

            if !prim.index.is_empty() {
                for tri in &prim.index {
                    let [a, b, c] = tri.idx;
                    flat_indices_u32.push(a + base_vertex);
                    flat_indices_u32.push(b + base_vertex);
                    flat_indices_u32.push(c + base_vertex);
                }
            }
            base_vertex += prim.vertex.len() as u32;
        }

        let vertex_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("mesh:{}:vertex", name)),
                contents: bytemuck::cast_slice(&flat_vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let index_buf = if flat_indices_u32.is_empty() {
            None
        } else {
            Some(
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("mesh:{}:index", name)),
                        contents: bytemuck::cast_slice(&flat_indices_u32),
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    }),
            )
        };

        let mesh = Mesh {
            name: Some(name.to_string()),
            primitives,
            vertex_buf,
            index_buf,
        };

        let id = self.meshes.insert(mesh);
        self.meshes_by_name.insert(name.to_string(), id);
        id
    }
}

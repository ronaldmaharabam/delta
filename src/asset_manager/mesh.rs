use super::{AssetManager, MeshId, material::MaterialId};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub const MAX_OBJECTS: usize = 10000;

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    pub const ATTRS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x2,
    ];

    pub fn buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}

#[derive(Clone)]
pub struct Index {
    pub idx: [u32; 3],
}
#[derive(Clone)]
pub struct Primitive {
    pub vertex: Vec<Vertex>,
    pub index: Vec<Index>,
    pub material: Option<usize>,
}

pub struct PrimitiveRange {
    pub first_index: u32,
    pub index_count: u32,
    pub base_vertex: i32,

    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],

    pub material: MaterialId,
}

pub struct Mesh {
    pub name: Option<String>,
    pub primitives: Vec<PrimitiveRange>,
    pub vertex_buf: wgpu::Buffer,
    pub index_buf: Option<wgpu::Buffer>,
    pub index_format: Option<wgpu::IndexFormat>,
}

pub struct ObjectUniform {
    pub model: [[f32; 4]; 4],
}

impl AssetManager {
    pub fn get_mesh(&mut self, name: &str) -> MeshId {
        if let Some(&id) = self.meshes_by_name.get(name) {
            return id;
        }

        let (path, selector) = Self::split_key(name);

        let primitives: Vec<Primitive> = self.importer.load_mesh(path, selector);

        let mut flat_vertices: Vec<Vertex> = Vec::new();
        let mut flat_indices_u32: Vec<u32> = Vec::new();
        let mut prim_ranges: Vec<PrimitiveRange> = Vec::new();

        let mut base_vertex: u32 = 0;

        for prim in &primitives {
            let vcount = prim.vertex.len() as u32;

            flat_vertices.extend_from_slice(&prim.vertex);

            let mut min = [f32::INFINITY; 3];
            let mut max = [f32::NEG_INFINITY; 3];
            for v in &prim.vertex {
                let p = v.position;
                for k in 0..3 {
                    if p[k] < min[k] {
                        min[k] = p[k];
                    }
                    if p[k] > max[k] {
                        max[k] = p[k];
                    }
                }
            }

            // Append indices (if present)
            let first_index = flat_indices_u32.len() as u32;
            let mut index_count = 0u32;

            if !prim.index.is_empty() {
                for index in &prim.index {
                    let [a, b, c] = &index.idx;
                    flat_indices_u32.push(base_vertex + a);
                    flat_indices_u32.push(base_vertex + b);
                    flat_indices_u32.push(base_vertex + c);
                }
                index_count = (prim.index.len() * 3) as u32;
            } else {
                // for i in 0..(vcount / 3) {
                //     let a = base_vertex + i*3 + 0;
                //     let b = base_vertex + i*3 + 1;
                //     let c = base_vertex + i*3 + 2;
                //     flat_indices_u32.extend_from_slice(&[a,b,c]);
                // }
                // index_count = (vcount / 3) * 3;
            }

            let material = if let Some(mat) = prim.material {
                self.get_material(&format!("{}#{}", path, mat))
            } else {
                0.into()
            };
            prim_ranges.push(PrimitiveRange {
                first_index,
                index_count,
                base_vertex: base_vertex as i32,
                aabb_min: min,
                aabb_max: max,
                material,
            });

            base_vertex += vcount;
        }

        let vertex_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("mesh:{}:vertex", name)),
                contents: bytemuck::cast_slice(&flat_vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let (index_buf, index_format) = if flat_indices_u32.is_empty() {
            (None, None)
        } else {
            // Try to downcast to u16 if possible
            let can_u16 = base_vertex <= 0x10000 && flat_indices_u32.iter().all(|&i| i < 0x10000);

            if can_u16 {
                let inds_u16: Vec<u16> = flat_indices_u32.iter().map(|&i| i as u16).collect();
                let ib = self
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("mesh:{}:index(u16)", name)),
                        contents: bytemuck::cast_slice(&inds_u16),
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    });
                (Some(ib), Some(wgpu::IndexFormat::Uint16))
            } else {
                let ib = self
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("mesh:{}:index(u32)", name)),
                        contents: bytemuck::cast_slice(&flat_indices_u32),
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    });
                (Some(ib), Some(wgpu::IndexFormat::Uint32))
            }
        };

        let mesh = Mesh {
            name: Some(name.to_string()),
            primitives: prim_ranges,
            vertex_buf,
            index_buf,
            index_format,
        };

        let id = self.meshes.insert(mesh);
        self.meshes_by_name.insert(name.to_string(), id);
        id
    }
    pub fn mesh(&self, key: MeshId) -> Option<&Mesh> {
        self.meshes.get(key)
    }
}

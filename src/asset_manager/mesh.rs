use super::{AssetManager, MeshId, material::MaterialId};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub const MAX_OBJECTS: usize = 10000;

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
}

impl Vertex {
    pub const ATTRS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Float32x3,
        3 => Float32x4,
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
    pub vertex_count: u32,
    pub index_count: u32,
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
        let id = self.set_mesh(&primitives, name);
        for (idx, prim) in primitives.iter().enumerate() {
            let material = if let Some(mat) = prim.material {
                self.get_material(&format!("{}#{}", path, mat))
            } else {
                0.into()
            };
            self.set_mat(id, idx, material);
        }
        id
    }

    pub fn set_mesh(&mut self, primitives: &[Primitive], name: &str) -> MeshId {
        let mut flat_vertices: Vec<Vertex> = Vec::new();
        let mut flat_indices_u32: Vec<u32> = Vec::new();
        let mut prim_ranges: Vec<PrimitiveRange> = Vec::new();

        let mut base_vertex: u32 = 0;

        for prim in primitives {
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
            }

            //let material = if let Some(mat) = prim.material {
            //    self.get_material(&format!("{}#{}", path, mat))
            //} else {
            //    0.into()
            //};
            prim_ranges.push(PrimitiveRange {
                first_index,
                index_count,
                base_vertex: base_vertex as i32,
                aabb_min: min,
                aabb_max: max,
                material: 0.into(),
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
            vertex_count: base_vertex,
            index_count: flat_indices_u32.len() as u32,
        };

        let id = self.meshes.insert(mesh);
        self.meshes_by_name.insert(name.to_string(), id);
        id
    }

    pub fn rewrite_mesh(&mut self, mesh_id: MeshId, primitives: &[Primitive]) {
        let mesh = self.meshes.get_mut(mesh_id).expect("invalid mesh_id");

        let mut flat_vertices: Vec<Vertex> = Vec::new();
        let mut flat_indices_u32: Vec<u32> = Vec::new();
        let mut base_vertex: u32 = 0;

        for prim in primitives {
            let vcount = prim.vertex.len() as u32;
            flat_vertices.extend_from_slice(&prim.vertex);

            if !prim.index.is_empty() {
                for index in &prim.index {
                    let [a, b, c] = index.idx;
                    flat_indices_u32.push(base_vertex + a);
                    flat_indices_u32.push(base_vertex + b);
                    flat_indices_u32.push(base_vertex + c);
                }
            }

            base_vertex += vcount;
        }

        assert_eq!(
            base_vertex, mesh.vertex_count,
            "vertex count mismatch on rewrite"
        );
        assert_eq!(
            flat_indices_u32.len() as u32,
            mesh.index_count,
            "index count mismatch on rewrite"
        );

        self.queue
            .write_buffer(&mesh.vertex_buf, 0, bytemuck::cast_slice(&flat_vertices));

        if let Some(ref ib) = mesh.index_buf {
            match mesh.index_format {
                Some(wgpu::IndexFormat::Uint16) => {
                    let inds_u16: Vec<u16> = flat_indices_u32.iter().map(|&i| i as u16).collect();
                    self.queue
                        .write_buffer(ib, 0, bytemuck::cast_slice(&inds_u16));
                }
                Some(wgpu::IndexFormat::Uint32) => {
                    self.queue
                        .write_buffer(ib, 0, bytemuck::cast_slice(&flat_indices_u32));
                }
                None => {}
            }
        }

        let mut prim_ranges: Vec<PrimitiveRange> = Vec::new();
        let mut cur_first_index = 0u32;
        let mut cur_base = 0i32;
        for prim in primitives {
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

            let index_count = (prim.index.len() * 3) as u32;

            prim_ranges.push(PrimitiveRange {
                first_index: cur_first_index,
                index_count,
                base_vertex: cur_base,
                aabb_min: min,
                aabb_max: max,
                material: 0.into(),
            });

            cur_first_index += index_count;
            cur_base += prim.vertex.len() as i32;
        }

        mesh.primitives = prim_ranges;
    }

    pub fn set_mat(&mut self, mesh_id: MeshId, idx: usize, mat_id: MaterialId) {
        if let Some(mesh) = self.meshes.get_mut(mesh_id) {
            assert!(
                idx < mesh.primitives.len(),
                "set_mat: primitive index {idx} out of range for mesh {:?}",
                mesh.name
            );
            mesh.primitives[idx].material = mat_id;
        } else {
            panic!("set_mat: mesh_id {:?} not found", mesh_id);
        }
    }

    pub fn mesh(&self, key: MeshId) -> Option<&Mesh> {
        self.meshes.get(key)
    }
}

use anyhow::Result;
use async_trait::async_trait;
use glam::{Mat4, Vec3};
use std::num::NonZeroU64;
use std::sync::Arc;
use wgpu::StoreOp;
use wgpu::util::DeviceExt;
use winit::window::Window;

use gpu::GpuContext;

use crate::asset_manager::AssetManager;
use crate::asset_manager::MeshId;
use crate::asset_manager::light::{Light, LightParams, LightUniform, MAX_LIGHTS};
use crate::asset_manager::material::MAX_MAT;
use crate::asset_manager::material::MatId;
use crate::asset_manager::mesh::{Mesh, Vertex};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub mod gpu;

pub struct RenderResource(wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroupLayout);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4], // 64 bytes
    pub camera_pos: [f32; 3],     // 12 bytes
    pub _pad0: f32,               // 4 bytes padding -> align to 16
}

impl CameraUniform {
    pub fn identity() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 0.0],
            _pad0: 0.0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y_radians: f32,
    pub z_near: f32,
    pub z_far: f32,
    pub aspect: f32,
}

impl Camera {
    pub fn view_proj(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);
        let proj = Mat4::perspective_rh(self.fov_y_radians, self.aspect, self.z_near, self.z_far);
        proj * view
    }
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: Vec3::new(0.0, 5.0, 10.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov_y_radians: std::f32::consts::FRAC_PI_4, // 45 degrees
            z_near: 0.1,
            z_far: 1000.0,
            aspect: 16.0 / 9.0,
        }
    }
}

pub struct RenderCommand {
    pub mesh_id: MeshId,
}

pub struct Command {
    pub mesh_ids: Vec<MeshId>,
    pub transforms: Vec<[[f32; 4]; 4]>,
}

pub struct ForwardRenderer {
    pub context: gpu::GpuContext,
    pub asset: AssetManager,
    pub pipeline: wgpu::RenderPipeline,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bg: wgpu::BindGroup,
    pub camera_bgl: wgpu::BindGroupLayout,

    pub camera: Camera,

    pub depth_tex: wgpu::Texture,
    pub depth_view: wgpu::TextureView,

    pub light_ssbo: wgpu::Buffer,
    pub light_params: wgpu::Buffer,
    pub light_bg: wgpu::BindGroup,
    pub light_bgl: wgpu::BindGroupLayout,

    pub mat_bg: wgpu::BindGroup,
    pub mat_bgl: wgpu::BindGroupLayout,

    pub mat_id_buffer: wgpu::Buffer,
    pub mat_id_bgl: wgpu::BindGroupLayout,
    pub mat_id_bg: wgpu::BindGroup,
}

impl ForwardRenderer {
    pub async fn new(window: &Arc<Window>) -> Result<Self> {
        let ctx = GpuContext::new(window).await?;

        let asset = AssetManager::new(ctx.device.clone(), ctx.queue.clone());

        // asset

        let mat_bgl = ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Material BGL"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let mat_bg = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material BG"),
            layout: &mat_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: asset.mat_buffer.as_entire_binding(),
            }],
        });

        let (camera_buffer, camera_bgl, camera_bg) = Self::create_camera(&ctx.device);
        let (light_ssbo, light_params, light_bgl, light_bg) =
            Self::create_light(&ctx.device, MAX_LIGHTS);

        let (mat_id_buffer, mat_id_bgl, mat_id_bg) =
            Self::create_material_id(&ctx.device, &ctx.queue, MAX_MAT);

        let depth_tex = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth"),
            size: wgpu::Extent3d {
                width: ctx.config.width,
                height: ctx.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let pipeline = {
            let shader = ctx
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Forward Shader"),
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                        "../shaders/forward.wgsl"
                    ))),
                });

            let vertex_layout = Vertex::buffer_layout();

            let pipeline_layout =
                ctx.device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Forward Pipeline Layout"),
                        bind_group_layouts: &[&camera_bgl, &light_bgl, &mat_bgl, &mat_id_bgl],
                        push_constant_ranges: &[],
                    });

            ctx.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Forward Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[vertex_layout],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: ctx.config.format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: DEPTH_FORMAT,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                })
        };

        Ok(Self {
            context: ctx,
            asset,
            pipeline,
            camera_buffer,
            camera_bg,
            camera_bgl,
            camera: Camera {
                eye: Vec3::new(0.0, 0.0, 5.0),
                target: Vec3::ZERO,
                up: Vec3::Y,
                fov_y_radians: 60.0f32.to_radians(),
                z_near: 0.1,
                z_far: 100.0,
                aspect: 1.0,
            },
            depth_tex,
            depth_view,
            light_ssbo,
            light_params,
            light_bg,
            light_bgl,
            mat_bg,
            mat_bgl,
            mat_id_buffer,
            mat_id_bgl,
            mat_id_bg,
        })
    }
    pub fn render(&mut self, lights: &[Light], cam: &Camera, action: &[RenderCommand]) {
        self.camera = cam.clone();
        self.update_camera_buffer();

        let ctx = &self.context;
        let device = &ctx.device;
        let queue = &ctx.queue;

        //let vp = cam.view_proj();
        //let cu = CameraUniform {
        //    view_proj: vp.to_cols_array_2d(),
        //};
        //queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&cu));

        let frame = match ctx.surface.get_current_texture() {
            Ok(f) => f,
            Err(err) => {
                match err {
                    wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                        ctx.surface.configure(&ctx.device, &ctx.config);
                    }
                    wgpu::SurfaceError::OutOfMemory => {
                        eprintln!("wgpu: OutOfMemory on surface get_current_texture");
                        std::process::exit(1);
                    }
                    wgpu::SurfaceError::Timeout => {
                        eprintln!("wgpu: surface acquire timeout; skipping frame");
                    }
                    _ => {
                        eprintln!("wgpu: surface acquire error; skipping frame");
                    }
                }
                return;
            }
        };

        let color_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = &self.depth_view;

        // upload lights
        {
            let light_buf = &self.light_ssbo;
            let params_buf = &self.light_params;

            let count = lights.len().min(MAX_LIGHTS);
            let mut tmp: Vec<LightUniform> = Vec::with_capacity(count);
            for l in lights.iter().take(count) {
                tmp.push(l.into());
            }

            if count > 0 {
                queue.write_buffer(light_buf, 0, bytemuck::cast_slice(&tmp));
            }

            let params = LightParams {
                count: count as u32,
                _pad: [0; 3],
            };
            queue.write_buffer(params_buf, 0, bytemuck::bytes_of(&params));
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Forward Encoder"),
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Forward Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.camera_bg, &[]);
            rpass.set_bind_group(1, &self.light_bg, &[]);
            rpass.set_bind_group(2, &self.mat_bg, &[]);

            for cmd in action {
                let mesh: &Mesh = self.asset.mesh(cmd.mesh_id).expect("mesh not found");

                rpass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));

                if let (Some(index_buf), Some(index_fmt)) =
                    (mesh.index_buf.as_ref(), mesh.index_format)
                {
                    rpass.set_index_buffer(index_buf.slice(..), index_fmt);

                    for p in &mesh.primitives {
                        //let mat_id: u32 = p.material.0 as u32;
                        let offset = (p.material.0 * std::mem::size_of::<MatId>()) as u32;

                        //queue.write_buffer(&self.mat_id_buffer, 0, bytemuck::bytes_of(&mat_id));
                        rpass.set_bind_group(3, &self.mat_id_bg, &[offset]);
                        let first = p.first_index;
                        let count = p.index_count;
                        rpass.draw_indexed(first..first + count, p.base_vertex, 0..1);
                    }
                }
            }
        }

        queue.submit(Some(encoder.finish()));
        frame.present();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        let ctx = &mut self.context;
        ctx.config.width = width;
        ctx.config.height = height;
        ctx.surface.configure(&ctx.device, &ctx.config);

        self.depth_tex = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.depth_view = self
            .depth_tex
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.camera.aspect = width as f32 / height as f32;
        self.update_camera_buffer();
    }

    pub fn update_camera_buffer(&mut self) {
        let vp = self.camera.view_proj();
        let cu = CameraUniform {
            view_proj: vp.to_cols_array_2d(),
            camera_pos: self.camera.eye.to_array(), // assuming glam::Vec3
            _pad0: 0.0,
        };
        self.context
            .queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&cu));
    }
    pub fn create_light(
        device: &wgpu::Device,
        max_lights: usize,
    ) -> (
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::BindGroupLayout,
        wgpu::BindGroup,
    ) {
        let light_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Light BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(
                                std::mem::size_of::<LightParams>() as u64
                            ),
                        },
                        count: None,
                    },
                ],
            });

        let limits = device.limits();
        let light_stride = std::mem::size_of::<LightUniform>() as u64;
        let mut lights_size = (max_lights as u64).saturating_mul(light_stride);

        let max_storage_bytes =
            (limits.max_storage_buffer_binding_size as u64).min(limits.max_buffer_size);

        lights_size = lights_size.min(max_storage_bytes);

        let lights_ssbo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Lights SSBO"),
            size: lights_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_ubo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Light Params UBO"),
            size: std::mem::size_of::<LightParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let light_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light BG"),
            layout: &light_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: lights_ssbo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_ubo.as_entire_binding(),
                },
            ],
        });

        (lights_ssbo, params_ubo, light_bgl, light_bg)
    }

    pub fn create_camera(
        device: &wgpu::Device,
    ) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let cam = CameraUniform::identity();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::bytes_of(&cam),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        (camera_buffer, camera_bgl, camera_bg)
    }
    pub fn create_material_id(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        max_ids: usize,
    ) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        // Fill [0, 1, 2, …, max_ids-1]
        let mat_ids: Vec<MatId> = (0..max_ids as u32)
            .map(|i| MatId {
                id: i,
                _pad: [0; 63],
            })
            .collect();

        let size = (mat_ids.len() * std::mem::size_of::<MatId>()) as wgpu::BufferAddress;

        let material_id_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Material ID Buffer"),
            size: size.next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Upload once
        queue.write_buffer(&material_id_buffer, 0, bytemuck::cast_slice(&mat_ids));

        let material_id_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Material ID BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: NonZeroU64::new(32), // must match WGSL struct size
                },
                count: None,
            }],
        });
        let material_id_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material ID BG"),
            layout: &material_id_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &material_id_buffer,
                    offset: 0,
                    size: NonZeroU64::new(std::mem::size_of::<MatId>() as u64),
                }),
            }],
        });

        (material_id_buffer, material_id_bgl, material_id_bg)
    }
}

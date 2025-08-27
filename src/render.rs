use std::sync::Arc;

use gpu::GpuContext;

use wgpu::StoreOp;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::asset_manager::MeshId;
use crate::asset_manager::mesh::{Mesh, Vertex};
use crate::asset_manager::{AssetManager, importer::Importer};

pub mod gpu;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn identity() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

use glam::{Mat4, Vec3};

const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 1.0,
]);

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
        OPENGL_TO_WGPU_MATRIX * proj * view
    }
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct RenderCommand {
    pub mesh_id: MeshId,
}

pub trait Renderer {
    fn init(&mut self, _window: &Arc<Window>) {}
    fn render(&mut self, _cmds: &[RenderCommand]) {}
    fn resize(&mut self, _width: u32, _height: u32) {}
}

pub struct ForwardRenderer<I: Importer> {
    pub context: Option<gpu::GpuContext>,

    pub asset: Option<AssetManager<I>>,
    pub pipeline: Option<wgpu::RenderPipeline>,
    pub camera_buffer: Option<wgpu::Buffer>,
    pub camera_bind_group: Option<wgpu::BindGroup>,
    pub camera_bind_group_layout: Option<wgpu::BindGroupLayout>,

    pub camera: Option<Camera>,

    pub depth_tex: Option<wgpu::Texture>,
    pub depth_view: Option<wgpu::TextureView>,
}

impl<I: Importer> Renderer for ForwardRenderer<I> {
    fn init(&mut self, window: &Arc<Window>) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let ctx = pollster::block_on(GpuContext::new(&window)).expect("failed to init gup");
            self.asset = Some(AssetManager::<I>::new(
                ctx.device.clone(),
                ctx.queue.clone(),
            ));
            self.context = Some(ctx);
            self.setup();
        }

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen_futures::spawn_local;

            let window_clone = window.clone();

            spawn_local(async move {
                let ctx = GpuContext::new(window_clone.clone()).await;
                self.asset = Some(AssetManager::<I>::new(
                    ctx.device.clone(),
                    ctx.queue.clone(),
                ));
                self.context = Some(ctx.expect("failed to init gpu"));
                self.setup();
            });
        }
    }

    fn render(&mut self, cmds: &[RenderCommand]) {
        let ctx = match self.context.as_ref() {
            Some(c) => c,
            None => return, // not initialized yet
        };

        let device = &ctx.device;
        let queue = &ctx.queue;

        // --- acquire the current frame ---
        let frame = match ctx.surface.get_current_texture() {
            Ok(f) => f,
            Err(err) => {
                match err {
                    wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                        // Reconfigure the surface and try again next frame.
                        ctx.surface.configure(&ctx.device, &ctx.config);
                    }
                    wgpu::SurfaceError::OutOfMemory => {
                        // Fatal: best to abort.
                        eprintln!("wgpu: OutOfMemory on surface get_current_texture");
                        std::process::exit(1);
                    }
                    wgpu::SurfaceError::Timeout => {
                        // Non-fatal, just skip this frame.
                        eprintln!("wgpu: surface acquire timeout; skipping frame");
                    }
                    _ => {
                        eprintln!("wgpu: surface acquire timeout; skipping frame");
                    }
                }
                return;
            }
        };

        let color_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = match self.depth_view.as_ref() {
            Some(v) => v,
            None => {
                frame.present();
                return;
            }
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Forward Encoder"),
        });

        // begin render pass
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

            rpass.set_pipeline(self.pipeline.as_ref().expect("pipeline not set"));
            rpass.set_bind_group(0, self.camera_bind_group.as_ref().expect("camera bg"), &[]);

            let asset = self.asset.as_ref().expect("asset manager not set");

            for cmd in cmds {
                let mesh: &Mesh = asset
                    .mesh(cmd.mesh_id)
                    .expect("mesh id not found in AssetManager");

                rpass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));

                if let (Some(index_buf), Some(index_fmt)) =
                    (mesh.index_buf.as_ref(), mesh.index_format)
                {
                    rpass.set_index_buffer(index_buf.slice(..), index_fmt);

                    for p in &mesh.primitives {
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
    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        let ctx = self.context.as_mut().expect("GPU context not initialized");
        ctx.config.width = width;
        ctx.config.height = height;
        ctx.surface.configure(&ctx.device, &ctx.config);

        // recreate depth
        let depth_tex = ctx.device.create_texture(&wgpu::TextureDescriptor {
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
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());
        self.depth_tex = Some(depth_tex);
        self.depth_view = Some(depth_view);

        // update camera aspect & upload
        if let Some(cam) = self.camera.as_mut() {
            cam.aspect = width as f32 / height as f32;
            self.update_camera_buffer();
        }
    }
}

impl<I: Importer> ForwardRenderer<I> {
    pub fn new() -> Self {
        Self {
            context: None,
            asset: None,
            pipeline: None,
            camera_buffer: None,
            camera_bind_group: None,
            camera_bind_group_layout: None,
            camera: None,
            depth_tex: None,
            depth_view: None,
        }
    }
    pub fn setup(&mut self) {
        let ctx = self.context.as_ref().expect("GPU context not initialized");
        let device = &ctx.device;
        let queue = &ctx.queue;

        let surface_format = ctx.config.format;

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

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let depth_tex = device.create_texture(&wgpu::TextureDescriptor {
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Forward Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "../shaders/forward.wgsl"
            ))),
        });

        let vertex_layout = Vertex::buffer_layout();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Forward Pipeline Layout"),
            bind_group_layouts: &[&camera_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },

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
        });

        self.camera_buffer = Some(camera_buffer);
        self.camera_bind_group_layout = Some(camera_bgl);
        self.camera_bind_group = Some(camera_bind_group);
        self.depth_view = Some(depth_view);
        self.depth_tex = Some(depth_tex);
        self.pipeline = Some(pipeline);
    }
    pub fn setup_camera(
        &mut self,
        eye: [f32; 3],
        target: [f32; 3],
        up: [f32; 3],
        fov_y_degrees: f32,
        z_near: f32,
        z_far: f32,
    ) {
        let ctx = self.context.as_ref().expect("GPU context not initialized");

        let aspect = ctx.config.width as f32 / ctx.config.height as f32;
        let cam = Camera {
            eye: Vec3::from(eye),
            target: Vec3::from(target),
            up: Vec3::from(up),
            fov_y_radians: f32::to_radians(fov_y_degrees),
            z_near,
            z_far,
            aspect,
        };

        self.camera = Some(cam);
        self.update_camera_buffer();
    }

    /// Recompute view-proj and write to GPU buffer (if camera exists).
    pub fn update_camera_buffer(&mut self) {
        let (ctx, cam, camera_buffer) = match (
            self.context.as_ref(),
            self.camera.as_ref(),
            self.camera_buffer.as_ref(),
        ) {
            (Some(ctx), Some(cam), Some(buf)) => (ctx, cam, buf),
            _ => return,
        };

        let vp = cam.view_proj();
        let cu = CameraUniform {
            view_proj: vp.to_cols_array_2d(),
        };
        ctx.queue
            .write_buffer(camera_buffer, 0, bytemuck::bytes_of(&cu));
    }
}

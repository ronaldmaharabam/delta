use std::sync::Arc;

use gpu::GpuContext;
use winit::window::Window;

pub mod gpu;

pub struct RenderCommand {}
pub trait Renderer {
    fn init_gpu(&mut self, _window: &Arc<Window>) {}
    fn render(&mut self, _cmds: Vec<RenderCommand>) {}
}

pub struct ForwardRenderer {
    pub context: Option<gpu::GpuContext>,

    //pub asset: Option<AssetManager>,
    pub pipeline: Option<wgpu::RenderPipeline>,
    pub camera_buffer: Option<wgpu::Buffer>,
    pub camera_bind_group: Option<wgpu::BindGroup>,

    pub depth_view: Option<wgpu::TextureView>,
}

impl Renderer for ForwardRenderer {
    fn init_gpu(&mut self, window: &Arc<Window>) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let ctx = pollster::block_on(GpuContext::new(&window));
            self.context = Some(ctx.expect("failed to init gpu"));
        }

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen_futures::spawn_local;

            let window_clone = window.clone();

            spawn_local(async move {
                let ctx = GpuContext::new(window_clone.clone()).await;
            });

            self.context = Some(ctx.expect("failed to init gpu"));
        }
    }
    fn render(&mut self, cmds: Vec<RenderCommand>) {}
}

impl ForwardRenderer {
    pub fn new() -> Self {
        Self {
            context: None,
            //asset: None,
            pipeline: None,
            camera_buffer: None,
            camera_bind_group: None,
            depth_view: None,
        }
    }
}

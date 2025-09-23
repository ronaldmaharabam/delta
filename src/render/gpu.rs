use std::sync::Arc;
use winit::window::Window;
pub struct GpuContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
}

use anyhow::{Context, Result};
use winit::dpi::PhysicalSize;

impl GpuContext {
    pub async fn new(window: &Arc<Window>) -> Result<Self> {
        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .context("No suitable GPU adapters found on the system")?;

        //let required_limits = wgpu::Limits::default().using_resolution(adapter.limits());
        let adapter_limits = adapter.limits();

        let limits = wgpu::Limits {
            max_binding_array_elements_per_shader_stage: adapter_limits
                .max_binding_array_elements_per_shader_stage
                .min(8192),
            ..wgpu::Limits::downlevel_defaults().using_resolution(adapter_limits)
        };
        let features = wgpu::Features::TEXTURE_BINDING_ARRAY
            | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
            | wgpu::Features::TEXTURE_BINDING_ARRAY;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: features,
                required_limits: limits,
                ..Default::default()
            })
            .await
            .context("Failed to request device")?;

        let size: PhysicalSize<u32> = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let present_mode = if surface_caps
            .present_modes
            .contains(&wgpu::PresentMode::Fifo)
        {
            wgpu::PresentMode::Fifo
        } else {
            surface_caps.present_modes[0]
        };

        let alpha_mode = surface_caps.alpha_modes[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            surface,
            config,
        })
    }
}

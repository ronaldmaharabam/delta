use std::{marker::PhantomData, sync::Arc};

use hecs::World;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop,
    window::Window,
};

use crate::{
    asset_manager::light::{Light, LightKind},
    render::{Camera, ForwardRenderer, RenderCommand},
};

pub struct App {
    pub window: Option<Arc<Window>>,
    pub world: World,
    pub renderer: Option<ForwardRenderer>,
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            #[cfg(not(target_arch = "wasm32"))]
            let window_attributes =
                winit::window::WindowAttributes::default().with_title("My Game");

            #[cfg(target_arch = "wasm32")]
            let window_attributes = {
                use wasm_bindgen::JsCast;
                use web_sys::HtmlCanvasElement;
                use winit::platform::web::WindowAttributesExtWebSys;

                let canvas = web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("my-canvas")
                    .unwrap()
                    .dyn_into::<HtmlCanvasElement>()
                    .unwrap();

                winit::window::WindowAttributes::default()
                    .with_title("My Game")
                    .with_canvas(Some(canvas))
            };

            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

            #[cfg(not(target_arch = "wasm32"))]
            {
                let renderer = pollster::block_on(ForwardRenderer::new(&window))
                    .expect("Failed to create renderer");
                self.renderer = Some(renderer);
            }

            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen_futures::spawn_local;
                let window_clone = window.clone();
                let renderer_slot = &mut self.renderer;

                spawn_local(async move {
                    let renderer = ForwardRenderer::new(&window_clone)
                        .await
                        .expect("Failed to create renderer");
                    *renderer_slot = Some(renderer);
                });
            }
            self.window = Some(window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    let asset = &mut renderer.asset;
                    let mesh_id = asset.get_mesh("meshes/sphere.glb#0");

                    let spotlight = Light {
                        kind: LightKind::Spot,
                        position: [0.0, 3.0, 0.0],
                        direction: [0.0, -1.0, 0.0],
                        color: [0.8, 0.8, 1.0],
                        range: 20.0,
                        inner_angle: 0.4,
                        outer_angle: 0.7,
                        ..Default::default()
                    };

                    let cam = Camera {
                        eye: glam::Vec3::new(0.0, 1.5, 5.0),
                        target: glam::Vec3::new(0.0, 0.0, 0.0),
                        up: glam::Vec3::new(0.0, 1.0, 0.0),
                        fov_y_radians: 60.0_f32.to_radians(),
                        z_near: 0.1,
                        z_far: 1000.0,
                        aspect: 16.0 / 9.0,
                    };

                    renderer.render(&[spotlight], &cam, &[RenderCommand { mesh_id }]);
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn new_events(&mut self, _: &ActiveEventLoop, _: winit::event::StartCause) {}
}

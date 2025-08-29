use std::sync::Arc;

use hecs::World;

use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::Window,
};

use crate::{
    asset_manager::{
        importer::Importer,
        light::{Light, LightKind},
    },
    render::{ForwardRenderer, RenderCommand, Renderer},
};

pub struct App<I: Importer> {
    pub window: Option<Arc<Window>>,
    pub world: World,
    //pub input: InputManager,
    pub renderer: ForwardRenderer<I>,
}

impl<I: Importer> ApplicationHandler for App<I> {
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
            self.renderer.init(&window);
            self.renderer.setup_camera(
                [0.0, 1.5, 5.0], // eye
                [0.0, 0.0, 0.0], // target
                [0.0, 1.0, 0.0], // up
                60.0,            // fov in degrees
                0.1,             // near
                1000.0,          // far
            );
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
                if let Some(asset) = self.renderer.asset.as_mut() {
                    let mesh_id = asset.get_mesh("meshes/cube.gltf#Cube");
                    let mut point_light = Light {
                        kind: LightKind::Point,
                        position: [0.0, 3.0, 0.0],
                        color: [1.0, 0.5, 0.5], // white
                        range: 15.0,
                        ..Default::default()
                    };

                    let spotlight = Light {
                        kind: LightKind::Spot,
                        position: [0.0, 3.0, 0.0],
                        direction: [0.0, -1.0, 0.0],
                        color: [0.8, 0.8, 1.0], // bluish
                        range: 20.0,
                        inner_angle: 0.4, // tighter inner cone
                        outer_angle: 0.7, // wider outer cone
                        ..Default::default()
                    };
                    let directional_light = Light {
                        kind: LightKind::Directional,
                        direction: [0.0, -1.0, 0.0], // pointing down
                        color: [1.0, 0.0, 0.0],      // red
                        ..Default::default()
                    };
                    self.renderer.render(
                        &[RenderCommand { mesh_id }],
                        &[point_light, spotlight, directional_light],
                    );
                }
            }
            WindowEvent::Resized(size) => {
                self.renderer.resize(size.width, size.height);
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

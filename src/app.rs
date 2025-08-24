use std::sync::Arc;

use hecs::World;

use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::Window,
};

pub struct App {
    pub window: Option<Arc<Window>>,
    pub world: World,
    //pub input: InputManager,
    //pub renderer: ForwardRenderer,
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

            self.window = Some(Arc::new(
                event_loop.create_window(window_attributes).unwrap(),
            ));

            #[cfg(not(target_arch = "wasm32"))]
            {
                //let ctx = pollster::block_on(GpuContext::new(window.clone()));
            }

            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen_futures::spawn_local;

                let window_clone = window.clone();

                let renderer_ptr: *mut ForwardRenderer = &mut self.renderer;

                spawn_local(async move {
                    //let ctx = GpuContext::new(window_clone.clone()).await;
                    //unsafe {
                    //    // give the renderer the context
                    //    (*renderer_ptr).init(ctx);
                    //}
                    //// nudge a redraw once the renderer is ready
                    //window_clone.request_redraw();
                });
            }
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

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn wasm_main() {
    use engine::core::run;

    console_error_panic_hook::set_once();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use engine::{app::App, asset_manager::importer::GltfImporter, render::ForwardRenderer};
    use winit::event_loop::{ControlFlow, EventLoop};
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let world = hecs::World::new();

    let mut app = App {
        world,
        renderer: None,
        window: None,
    };

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    #[cfg(not(target_arch = "wasm32"))]
    event_loop.run_app(&mut app).unwrap();

    #[cfg(target_arch = "wasm32")]
    event_loop.spawn_app(app);
}

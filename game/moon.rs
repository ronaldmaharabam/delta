use core::f32;

use engine::{
    asset_manager::{
        MeshId,
        light::{Light, LightKind},
        mesh::{Index, Primitive, Vertex},
    },
    game::Game,
    render::{Camera, ForwardRenderer, RenderCommand},
};
use glam::{Vec2, Vec3};
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
    use engine::app::App;
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

    let game = MoonGame::new();

    println!("here");

    let mut app = App {
        world,
        renderer: None,
        window: None,
        game,
    };

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    #[cfg(not(target_arch = "wasm32"))]
    event_loop.run_app(&mut app).unwrap();

    #[cfg(target_arch = "wasm32")]
    event_loop.spawn_app(app);
}

struct MoonGame {
    pub mesh: [Primitive; 9],
    pub cam: Camera,
    pub sun: Light,
    pub mesh_id: Option<MeshId>,
}
impl Game for MoonGame {
    fn setup(&mut self, _world: &mut hecs::World, renderer: &mut ForwardRenderer) {
        let id = renderer.asset.set_mesh(&self.mesh, "moon");
        self.mesh_id = Some(id);
    }
    fn update(&mut self, _world: &mut hecs::World, renderer: &mut ForwardRenderer) {
        if let Some(id) = self.mesh_id {
            let mesh_id = renderer.asset.get_mesh("");
            let action = &[
                //RenderCommand { mesh_id: id },
                RenderCommand { mesh_id: mesh_id },
            ];

            renderer.render(&[self.sun], &self.cam, action);
        }
    }
}
impl MoonGame {
    pub fn new() -> Self {
        let tile_size = 500.0_f32;
        let base_delta = 3.0_f32;
        let center_high_res_factor = 1.0_f32;

        let mut meshes: [Primitive; 9] = {
            let empty = Primitive {
                vertex: Vec::new(),
                index: Vec::new(),
                material: None,
            };
            [
                empty.clone(),
                empty.clone(),
                empty.clone(),
                empty.clone(),
                empty.clone(),
                empty.clone(),
                empty.clone(),
                empty.clone(),
                empty.clone(),
            ]
        };

        let mut idx = 0usize;
        for j in -1..=1 {
            for i in -1..=1 {
                let origin_x = i as f32 * tile_size - tile_size / 2.0;
                let origin_y = j as f32 * tile_size - tile_size / 2.0;
                let origin = Vec2::new(origin_x, origin_y);
                let end = Vec2::new(origin_x + tile_size, origin_y + tile_size);

                let delta = if i == 0 && j == 0 {
                    base_delta / center_high_res_factor
                } else {
                    base_delta
                };

                meshes[idx] = Self::grid(origin, end, delta);
                idx += 1;
            }
        }

        //let sun = Light {
        //    kind: LightKind::Spot,
        //    position: [0.0, 3.0, 0.0],
        //    direction: [0.0, -1.0, 0.0],
        //    color: [0.8, 0.8, 1.0],
        //    range: 20.0,
        //    inner_angle: 0.4,
        //    outer_angle: 0.7,
        //    ..Default::default()
        //};

        //let cam = Camera {
        //    eye: Vec3::new(0.0, 0.0, 5.0),
        //    target: Vec3::ZERO,
        //    up: Vec3::Y,
        //    fov_y_radians: 60.0f32.to_radians(),
        //    z_near: 0.1,
        //    z_far: 100.0,
        //    aspect: 1.0,
        //};

        let sun = Light {
            kind: LightKind::Directional,
            position: [0.0, 0.0, 0.0],
            direction: [0.0, -1.0, 0.1], // flip so it hits normals
            color: [1.0, 1.0, 1.0],
            range: f32::INFINITY,
            inner_angle: 0.0,
            outer_angle: 0.0,
        };

        let cam = Camera {
            eye: Vec3::new(0.0, 50.0, 50.0), // flip Z if needed
            target: Vec3::new(0.0, 0.0, 0.0),
            up: Vec3::Y,
            fov_y_radians: std::f32::consts::FRAC_PI_4,
            z_near: 0.1,
            z_far: 5000.0,
            aspect: 16.0 / 9.0,
        };
        Self {
            mesh: meshes,
            cam,
            sun,
            mesh_id: None,
        }
    }

    fn grid(mut origin: Vec2, mut end: Vec2, delta: f32) -> Primitive {
        assert!(delta > 0.0, "delta must be > 0");

        if origin.x > end.x {
            std::mem::swap(&mut origin.x, &mut end.x);
        }
        if origin.y > end.y {
            std::mem::swap(&mut origin.y, &mut end.y);
        }

        let span_x = end.x - origin.x;
        let span_y = end.y - origin.y;

        let steps_x = (span_x / delta).ceil() as usize;
        let steps_y = (span_y / delta).ceil() as usize;

        let width = steps_x + 1;
        let height = steps_y + 1;

        let mut vertex = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let px = origin.x + x as f32 * delta;
                let py = origin.y + y as f32 * delta;

                let px = if x == steps_x { end.x } else { px };
                let py = if y == steps_y { end.y } else { py };

                let u = (px - origin.x) / span_x;
                let v = (py - origin.y) / span_y;

                vertex.push(Vertex {
                    position: [px, 0.0, py],
                    normal: [0.0, 1.0, 0.0],
                    uv: [u, v],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                });
            }
        }

        let mut index = Vec::with_capacity((width - 1) * (height - 1) * 2);
        for y in 0..(height - 1) {
            for x in 0..(width - 1) {
                let i0 = (y * width + x) as u32;
                let i1 = (y * width + x + 1) as u32;
                let i2 = ((y + 1) * width + x) as u32;
                let i3 = ((y + 1) * width + x + 1) as u32;

                //index.push(Index { idx: [i0, i2, i1] });
                //index.push(Index { idx: [i1, i2, i3] });
                index.push(Index { idx: [i0, i1, i2] });
                index.push(Index { idx: [i1, i3, i2] });
            }
        }

        Primitive {
            vertex,
            index,
            material: None,
        }
    }
}

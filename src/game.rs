use hecs::World;

use crate::{
    asset_manager::light::{Light, LightKind},
    render::{Camera, ForwardRenderer, RenderCommand},
};

pub trait Game {
    fn setup(&mut self, world: &mut World, renderer: &mut ForwardRenderer) {}
    fn update(&mut self, world: &mut World, renderer: &mut ForwardRenderer) {}
}
impl Game for () {
    fn setup(&mut self, world: &mut World, renderer: &mut ForwardRenderer) {}
    fn update(&mut self, world: &mut World, renderer: &mut ForwardRenderer) {
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

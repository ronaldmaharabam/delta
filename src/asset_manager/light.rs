pub const MAX_LIGHTS: usize = 16;

#[derive(Clone, Copy, Debug)]
pub enum LightKind {
    Point,
    Directional,
    Spot,
}

#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub kind: LightKind,

    pub position: [f32; 3],

    pub direction: [f32; 3],

    pub color: [f32; 3],

    pub range: f32,

    pub inner_angle: f32,

    pub outer_angle: f32,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            kind: LightKind::Point,
            position: [0.0, 0.0, 0.0],
            direction: [0.0, -1.0, 0.0],
            color: [1.0, 1.0, 1.0],
            range: 10.0,
            inner_angle: 0.5, // ~30 deg
            outer_angle: 0.7, // ~40 deg
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 3],
    pub _pad0: f32,

    pub color: [f32; 3],
    pub _pad1: f32,

    pub direction: [f32; 3],
    pub light_type: u32,

    pub range: f32,
    pub inner_cos: f32,
    pub outer_cos: f32,
    pub _pad2: f32,
}

impl From<&Light> for LightUniform {
    fn from(l: &Light) -> Self {
        let kind = match l.kind {
            LightKind::Point => 0,
            LightKind::Directional => 1,
            LightKind::Spot => 2,
        };

        Self {
            position: l.position,
            _pad0: 0.0,

            color: l.color,
            _pad1: 0.0,

            direction: l.direction,
            light_type: kind,

            range: l.range,
            inner_cos: l.inner_angle.cos(),
            outer_cos: l.outer_angle.cos(),
            _pad2: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightParams {
    pub count: u32,
    pub _pad: [u32; 3],
}

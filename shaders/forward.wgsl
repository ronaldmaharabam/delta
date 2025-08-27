// shaders/forward.wgsl
struct Camera {
    view_proj : mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera : Camera;

struct VSIn {
    @location(0) position : vec3<f32>,
    @location(1) uv       : vec2<f32>,
};

struct VSOut {
    @builtin(position) pos : vec4<f32>,
    @location(0) uv        : vec2<f32>,
};

@vertex
fn vs_main(in: VSIn) -> VSOut {
    var out: VSOut;
    out.pos = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    // flat color; wire in textures later
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}


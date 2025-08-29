// shaders/forward.wgsl

const MAX_LIGHTS : u32 = 16u;

struct Camera {
    view_proj : mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera : Camera;

// ---- Lights ----
struct GpuLight {
    position   : vec3<f32>,  _pad0 : f32,
    color      : vec3<f32>,  _pad1 : f32,
    direction  : vec3<f32>,  light_type : u32, // 0=Point,1=Directional,2=Spot
    range      : f32,
    inner_cos  : f32,
    outer_cos  : f32,
    _pad2      : f32,
};

struct LightBuffer {
    lights : array<GpuLight, MAX_LIGHTS>,
};

struct LightParams {
    count : u32,
    //_pad  : vec3<u32>,
};

@group(1) @binding(0)
var<storage, read> u_lights : LightBuffer;

@group(1) @binding(1)
var<uniform> u_lightParams : LightParams;

// ---- Vertex I/O ----
struct VSIn {
    @location(0) position : vec3<f32>,
    @location(1) uv       : vec2<f32>,
    @location(2) normal   : vec3<f32>,   // <-- add this
};

struct VSOut {
    @builtin(position) pos_clip : vec4<f32>,
    @location(0) uv             : vec2<f32>,
    @location(1) pos_ws         : vec3<f32>,
    @location(2) n_ws           : vec3<f32>,
};

@vertex
fn vs_main(in: VSIn) -> VSOut {
    var out: VSOut;
    // Assuming positions/normals are already in world space (no model matrix yet)
    out.pos_clip = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.uv       = in.uv;
    out.pos_ws   = in.position;
    out.n_ws     = normalize(in.normal);
    return out;
}

// Simple helpers
fn saturate(x: f32) -> f32 { return clamp(x, 0.0, 1.0); }

fn lambert(n: vec3<f32>, l: vec3<f32>) -> f32 {
    return max(dot(n, l), 0.0);
}

// Attenuation for point/spot (smooth-ish, range-based)
fn range_atten(dist: f32, range: f32) -> f32 {
    let x = saturate(1.0 - dist / max(range, 1e-3));
    // smoother falloff
    return x * x;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    // temporary flat albedo; plug in textures later
    let albedo = vec3<f32>(1.0, 1.0, 1.0);

    var color = vec3<f32>(0.0);

    let N = normalize(in.n_ws);

    // Accumulate all lights
    let count = min(u_lightParams.count, MAX_LIGHTS);
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let Ld = u_lights.lights[i];

        // Build the light direction and attenuation based on type
        var L : vec3<f32>;
        var att : f32 = 1.0;

        if (Ld.light_type == 0u) { // Point
            let toL   = Ld.position - in.pos_ws;
            let dist  = length(toL);
            L         = normalize(toL);
            att       = range_atten(dist, Ld.range);
        } else if (Ld.light_type == 1u) { // Directional
            // direction points *from* light towards scene; ensure normalized
            L = normalize(-Ld.direction);
        } else { // Spot
            let toL   = Ld.position - in.pos_ws;
            let dist  = length(toL);
            L         = normalize(toL);
            let spotC = dot(-L, normalize(Ld.direction)); // angle from cone axis
            // soft cone using smoothstep between outer and inner
            let cone  = saturate((spotC - Ld.outer_cos) / max(Ld.inner_cos - Ld.outer_cos, 1e-4));
            att       = range_atten(dist, Ld.range) * cone;
        }

        let ndotl = lambert(N, L);
        color += Ld.color * albedo * ndotl * att;
    }

    return vec4<f32>(color, 1.0);
}


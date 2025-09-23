// shaders/forward.wgsl
const MAX_LIGHTS : u32 = 16u;
const PI : f32 = 3.14159265359;

struct Camera {
    view_proj : mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad0     : f32,
};

@group(0) @binding(0)
var<uniform> camera : Camera;

// ---- Lights ----
struct GpuLight {
    position   : vec3<f32>,  _pad0 : f32,
    color      : vec3<f32>,  _pad1 : f32,
    direction  : vec3<f32>,  light_type : u32,
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
};

@group(1) @binding(0)
var<storage, read> u_lights : LightBuffer;
@group(1) @binding(1)
var<uniform> u_lightParams : LightParams;

// ---- Materials ----
struct Material {
    base_color_factor : vec4<f32>,
    emissive_factor   : vec3<f32>,
    emissive_padding  : f32,
    metallic_factor   : f32,
    roughness_factor  : f32,
    alpha_cutoff      : f32,
    double_sided      : u32,
    texture_indices   : array<i32, 4>,
};

@group(2) @binding(0)
var<storage, read> materials : array<Material>;

// ---- Per-draw material ID ----
struct MaterialParams {
    id : u32,
    _pad: vec3<u32>, 

};

@group(3) @binding(0)
var<uniform> material_params : MaterialParams;

// ---- Vertex I/O ----
struct VSIn {
    @location(0) position : vec3<f32>,
    @location(1) uv       : vec2<f32>,
    @location(2) normal   : vec3<f32>,
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
    out.pos_clip = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.uv       = in.uv;
    out.pos_ws   = in.position;
    out.n_ws     = normalize(in.normal);
    return out;
}

// ---- Helpers ----
fn saturate(x: f32) -> f32 { return clamp(x, 0.0, 1.0); }

fn range_atten(dist: f32, range: f32) -> f32 {
    let x = saturate(1.0 - dist / max(range, 1e-3));
    return x * x;
}

// Fresnel Schlick
fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(1.0 - cos_theta, 5.0);
}

// GGX / Trowbridge-Reitz normal distribution
fn distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotH = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;
    let denom = (NdotH2 * (a2 - 1.0) + 1.0);
    return a2 / (PI * denom * denom + 1e-6);
}

// Schlick-GGX geometry (per-direction)
fn geometry_schlick_ggx(NdotV: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0; // UE4/roughness->k approximation
    return NdotV / (NdotV * (1.0 - k) + k + 1e-6);
}

// Smith geometry (combined)
fn geometry_smith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx1 = geometry_schlick_ggx(NdotV, roughness);
    let ggx2 = geometry_schlick_ggx(NdotL, roughness);
    return ggx1 * ggx2;
}

// Simple Reinhard tonemap
fn tonemap_reinhard(x: vec3<f32>) -> vec3<f32> {
    return x / (x + vec3<f32>(1.0));
}

// ---- Fragment ----
@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    // NOTE: CPU must ensure material_params.id is in-range.
    let mat = materials[material_params.id];

    // base_color_factor in glTF is linear already => don't pow
    let albedo : vec3<f32> = mat.base_color_factor.rgb;

    let metallic  = mat.metallic_factor;
    // clamp roughness to avoid singularities; allow very small values but not zero
    let roughness = clamp(mat.roughness_factor, 0.02, 1.0);

    let N = normalize(in.n_ws);

    // camera position provided in camera uniform
    let V = normalize(camera.camera_pos - in.pos_ws);
    var Lo = vec3<f32>(0.0);

    let count = min(u_lightParams.count, MAX_LIGHTS);
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let Ld = u_lights.lights[i];

        var L : vec3<f32>;
        var att : f32 = 1.0;

        if (Ld.light_type == 0u) { // Point
            let toL   = Ld.position - in.pos_ws;
            let dist  = length(toL);
            L         = normalize(toL);
            att       = range_atten(dist, Ld.range);
        } else if (Ld.light_type == 1u) { // Directional
            L = normalize(-Ld.direction);
        } else { // Spot
            let toL   = Ld.position - in.pos_ws;
            let dist  = length(toL);
            L         = normalize(toL);
            let spotC = dot(-L, normalize(Ld.direction));
            let cone  = saturate((spotC - Ld.outer_cos) / max(Ld.inner_cos - Ld.outer_cos, 1e-4));
            att       = range_atten(dist, Ld.range) * cone;
        }

        let H = normalize(V + L);

        let NDF = distribution_ggx(N, H, roughness);
        let G   = geometry_smith(N, V, L, roughness);

        // F0: dielectric default 0.04, lerp to albedo for metals
        var F0 = vec3<f32>(0.04);
        // metallic blends the F0 toward albedo (albedo should be linear)
        F0 = F0 + (albedo - F0) * metallic;

        let cosHV = max(dot(H, V), 0.0);
        let F = fresnel_schlick(cosHV, F0);

        let NdotV = max(dot(N, V), 0.0);
        let NdotL = max(dot(N, L), 0.0);

        let numerator   = NDF * G * F;
        let denominator = max(4.0 * NdotV * NdotL, 1e-6);
        let specular    = numerator / denominator;

        let kS = F;
        let kD = (vec3<f32>(1.0) - kS) * (1.0 - metallic);

        let diffuse = kD * albedo / PI;

        Lo += (diffuse + specular) * Ld.color * NdotL * att;
    }

    let emissive = mat.emissive_factor;

    // HDR accumulation -> simple tonemap -> gamma
    var color = Lo + emissive;
    color = tonemap_reinhard(color);
    color = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));

    return vec4<f32>(color, 1.0);
}


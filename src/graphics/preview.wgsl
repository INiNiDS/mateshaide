struct Uniforms {
    rect: vec4<f32>,
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    let r = uniforms.rect;
    var corners = array<vec2<f32>, 4>(
        vec2(r.x, r.y),
        vec2(r.z, r.y),
        vec2(r.x, r.w),
        vec2(r.z, r.w),
    );
    var uvs = array<vec2<f32>, 4>(
        vec2(0.0, 1.0),
        vec2(1.0, 1.0),
        vec2(0.0, 0.0),
        vec2(1.0, 0.0),
    );
    var idx_arr = array<u32, 6>(0u, 1u, 2u, 2u, 1u, 3u);
    let idx = idx_arr[vi];

    var out: VSOut;
    out.pos = vec4(corners[idx], 0.0, 1.0);
    out.uv = uvs[idx];
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let color = textureSample(tex, samp, in.uv);
    return vec4(color.rgb, color.a * uniforms.opacity);
}
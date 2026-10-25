struct U {
    progress: f32,
    intensity: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: U;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    var t = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0),
    );
    var o: VSOut;
    o.pos = vec4(t[vi], 0.0, 1.0);
    o.uv = t[vi] * 0.5 + 0.5;
    return o;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let uv = clamp(in.uv, vec2(0.0), vec2(1.0));
    let p = u.progress;
    let I = u.intensity;

    let c = vec2(0.5, 0.5);
    let g0 = mix(vec2(0.0, 0.0), c, p);
    let g1 = mix(vec2(1.0, 0.0), c, p);
    let g2 = mix(vec2(0.0, 1.0), c, p);
    let g3 = mix(vec2(1.0, 1.0), c, p);

    let glow = (
        exp(-length(uv - g0) * 4.5) +
        exp(-length(uv - g1) * 4.5) +
        exp(-length(uv - g2) * 4.5) +
        exp(-length(uv - g3) * 4.5)
    ) * 0.3;

    let de = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    let edge = exp(-de * 10.0) * 0.35;

    let total = (glow * (1.0 + p * 4.0) + edge) * I;

    let dark = (0.12 + 0.3 * p) * I;

    let blue = vec3(0.18, 0.45, 1.0);
    let hot  = vec3(0.95, 0.98, 1.0);
    let col  = mix(blue, hot, clamp(total, 0.0, 1.0));

    let a = clamp(total * 0.92 + dark * 0.5, 0.0, 0.96);
    return vec4(col * clamp(total * 2.2, 0.0, 1.0), a);
}

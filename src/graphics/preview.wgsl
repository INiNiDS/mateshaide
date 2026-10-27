struct U {
    screen: vec2<f32>,
    center: vec2<f32>,
    card:   vec2<f32>,
    photo:  vec2<f32>,
    frame:  vec4<f32>,
    anim:   vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VSOut { @builtin(position) pos: vec4<f32> };

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    var t = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    var o: VSOut;
    o.pos = vec4(t[vi], 0.0, 1.0);
    return o;
}

fn sd_round_box(p: vec2<f32>, h: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - h + vec2(r);
    return length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn hash21(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2(123.34, 456.21));
    q += dot(q, q + 45.32);
    return fract(q.x * q.y);
}

fn to_local(rel: vec2<f32>, ang: f32, sc: f32) -> vec2<f32> {
    let c = cos(ang);
    let s = sin(ang);
    return vec2(c * rel.x + s * rel.y, -s * rel.x + c * rel.y) / sc;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let side = u.frame.x;
    let bottom = u.frame.y;
    let ang = u.frame.z;
    let sc = max(u.frame.w, 0.01);
    let opacity = u.anim.x;
    let dev = u.anim.y;
    let shadow_k = u.anim.z;
    let dim = u.anim.w;

    let rel = in.pos.xy - u.center;
    let q = to_local(rel, ang, sc);
    let half = u.card * 0.5;

    let d = sd_round_box(q, half, 3.0);
    let cover = 1.0 - smoothstep(-0.75, 0.75, d);

    let blur = 18.0 + 30.0 * shadow_k;
    let qs = to_local(rel - vec2(0.0, 10.0 + 26.0 * shadow_k), ang, sc);
    let ds = sd_round_box(qs, half, 3.0);
    let shadow = (1.0 - smoothstep(-blur * 0.3, blur, ds)) * 0.55 * shadow_k;
    let bg_a = 1.0 - (1.0 - dim) * (1.0 - shadow);

    var paper = vec3(0.965, 0.955, 0.93);
    paper *= 1.0 - 0.04 * (q.y / u.card.y + 0.5);
    paper += (hash21(in.pos.xy) - 0.5) * 0.02;

    let pq = q - vec2(0.0, (side - bottom) * 0.5);
    let uv = pq / u.photo + 0.5;
    let inside = max(abs(pq.x) - u.photo.x * 0.5, abs(pq.y) - u.photo.y * 0.5);
    let img = textureSampleLevel(tex, samp, clamp(uv, vec2(0.0), vec2(1.0)), 0.0).rgb;

    let lum = dot(img, vec3(0.299, 0.587, 0.114));
    let t_b = smoothstep(0.0, 0.75, dev);
    let t_s = smoothstep(0.3, 1.0, dev);
    var photo = mix(vec3(lum), img, t_s);
    photo = mix(vec3(0.17, 0.19, 0.18), photo, t_b);
    let grain = (hash21(in.pos.xy + 7.0) - 0.5) * 0.05 * (1.0 - 0.6 * dev);
    let v = uv - 0.5;
    photo = (photo + grain) * (1.0 - 0.36 * dot(v, v));
    photo *= 0.82 + 0.18 * smoothstep(0.0, 5.0, -inside);

    let in_photo = 1.0 - smoothstep(-0.5, 0.5, inside);
    var col = mix(paper, photo, in_photo);

    col += smoothstep(0.35, 0.0, abs(q.x / u.card.x + q.y / u.card.y * 0.6 - 0.1)) * 0.05;

    let a = cover + bg_a * (1.0 - cover);
    let rgb = col * cover / max(a, 1e-4);
    return vec4(rgb, a * opacity);
}
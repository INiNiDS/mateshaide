@group(0) @binding(0) var image_texture: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;

struct VertexInput {
    @location(0) rect: vec4<f32>,
    @location(1) uv_rect: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
    input: VertexInput,
    @builtin(vertex_index) index: u32,
) -> VertexOutput {
    var corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0),
        vec2(1.0, 0.0),
        vec2(0.0, 1.0),
        vec2(0.0, 1.0),
        vec2(1.0, 0.0),
        vec2(1.0, 1.0),
    );

    let corner = corners[index];

    var output: VertexOutput;
    output.position = vec4(
        mix(input.rect.xy, input.rect.zw, corner),
        0.0,
        1.0,
    );
    output.uv = mix(
        input.uv_rect.xy,
        input.uv_rect.zw,
        corner,
    );

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(
        image_texture,
        image_sampler,
        input.uv,
    );
}

struct TextOutput {
    [[builtin(position)]] pos: vec4<f32>;
    // [[loca]]
    [[location(0)]] glyph_uv: vec2<f32>;
    [[location(1)]] color: vec4<f32>;
};

[[block]]
struct TextUniforms {
    proj: mat4x4<f32>;
    // color: vec4<f32>;
};

[[group(0), binding(0)]]
var<uniform> u: TextUniforms;

[[group(0), binding(1)]]
var tex: texture_2d<f32>;
[[group(0), binding(2)]]
var samplr: sampler;

[[stage(vertex)]]
fn vs_text_main(
    [[location(0)]] uv: vec2<f32>,
    [[location(1)]] pos: vec2<f32>,
    [[location(2)]] size: vec2<f32>,
    [[location(3)]] uv_extents: vec4<f32>,
    [[location(4)]] color: vec4<f32>,
) -> TextOutput {
    var out: TextOutput;
    out.glyph_uv = uv_extents.xy + (uv_extents.zw * uv);
    out.color = color;
    out.pos = u.proj * vec4<f32>(pos + (size * uv), 0.0, 1.0);
    return out;
}

[[stage(fragment)]]
fn fs_text_main(in: TextOutput) -> [[location(0)]] vec4<f32> {
    let glyph_alpha = in.color.x * textureSample(tex, samplr, in.glyph_uv).x;
    // if (a <= 0.01) {
        // discard;
    // }

    return vec4<f32>(in.color.xyz, glyph_alpha);

}
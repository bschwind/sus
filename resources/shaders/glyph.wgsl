
struct TextOutput {
    [[builtin(position)]] pos: vec4<f32>;
    // [[loca]]
    [[location(0)]] glyph_uv: vec2<f32>;
    [[location(1)]] glyph_color: vec4<f32>;
};

[[block]]
struct TextUniforms {
    proj: mat4x4<f32>;
    // color: vec4<f32>;
};

[[group(0), binding(0)]]
var<uniform> u: TextUniforms;

[[group(0), binding(1)]]
var glyph_texture: texture_2d<f32>;
[[group(0), binding(2)]]
var glyph_texture_sampler: sampler;

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
    out.glyph_color = color;
    out.pos = u.proj * vec4<f32>(pos + (size * uv), 0.0, 1.0);
    return out;
}

[[stage(fragment)]]
fn fs_text_main(in: TextOutput) -> [[location(0)]] vec4<f32> {
    // let glyph_alpha = in.color.x * textureSample(tex, samplr, in.glyph_uv).r;
    // if (a <= 0.01) {
        // discard;
    // }

    // return vec4<f32>(in.color.xyz, glyph_alpha);
    let glyph_alpha = textureSample(glyph_texture, glyph_texture_sampler, in.glyph_uv).r;
    // let glyph_alpha = texture(sampler2D(glyph_texture, glyph_texture_sampler), glyph_uv).r;
    // color_out = vec4(glyph_color.rgb, glyph_alpha * glyph_color.a);
    return vec4<f32>(in.glyph_color.rgb, in.glyph_color.a * glyph_alpha);

}
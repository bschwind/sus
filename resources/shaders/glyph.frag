#version 450


layout(set = 0, binding = 1) uniform texture2D glyph_texture;
layout(set = 0, binding = 2) uniform sampler glyph_texture_sampler;

// Input from vertex shader
layout(location = 0) in vec2 glyph_uv;
layout(location = 1) in vec4 glyph_color;

// Fragment shader output
layout(location = 0) out vec4 color_out;

void main() {
    float glyph_alpha = texture(sampler2D(glyph_texture, glyph_texture_sampler), glyph_uv).r;
    color_out = vec4(glyph_color.rgb, glyph_alpha * glyph_color.a);
}

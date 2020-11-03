#version 450

layout(set = 0, binding = 0) uniform Locals {
    mat4 proj;
};

// Normalized, default UV coordinates from the
// glpyh quad.
layout(location = 0) in vec2 uv;

// Attributes from the instance array
layout(location = 1) in vec2 pos;
layout(location = 2) in vec2 size; // (width, height)
layout(location = 3) in vec4 uv_extents; // (u, v, width, height), texture space
layout(location = 4) in vec4 color;

layout(location = 0) out vec2 glyph_uv;
layout(location = 1) out vec4 glyph_color;

void main() {
    glyph_uv = uv_extents.xy + (uv_extents.zw * uv);
    glyph_color = color;

    vec4 output_pos = vec4(pos + (size * uv), 0.0, 1.0);
    gl_Position = proj * output_pos;
}

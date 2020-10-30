#version 450

// Input from vertex shader
layout(location = 0) in vec2 vert_uv;

// Fragment shader output
layout(location = 0) out vec4 outColor;

void main() {
    outColor = vec4(vert_uv.x, vert_uv.y, 1.0, 1.0);
}

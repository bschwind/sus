#version 450

// Vertex attributes
layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;

// Shader output
layout(location = 0) out vec2 vert_uv;

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    vert_uv = uv;
    gl_Position = vec4(pos, 0.0, 1.0);
}

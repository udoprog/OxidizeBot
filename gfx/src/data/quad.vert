#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(constant_id = 0) const float scale = 1.0f;

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 0) out vec2 v_uv;

layout(set = 0, binding = 0) uniform SpriteData {
    /// dimensions of the image.
    vec2 dims;
    /// region of a single sprite.
    vec2 sprite_region;
    /// column to render.
    int column;
    /// row to render.
    int row;
} sprite_data;

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    float x = (sprite_data.sprite_region.x * (sprite_data.column + uv.x)) / sprite_data.dims.x;
    float y = (sprite_data.sprite_region.y * (sprite_data.row + uv.y)) / sprite_data.dims.y;

    v_uv = uv;
    gl_Position = vec4(scale * pos, 0.0, 1.0);
}

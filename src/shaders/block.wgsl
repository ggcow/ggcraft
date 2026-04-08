@group(1) @binding(0)
var<uniform> mvp: mat4x4<f32>;

struct VertexInput {
    @builtin(vertex_index) vertexid: u32,
    @location(0) cube_position: vec4<i32>,
    @location(1) cube_size: vec3<i32>,
    @location(2) tex_index: u32,
    @location(3) color: u32,
}

const M: array<mat3x3<f32>, 6> = array<mat3x3<f32>, 6>(
    mat3x3<f32>(0, 1, 0, 0, 0, 1, 0, 0, 0), //-x
    mat3x3<f32>(0, 0, 1, 0, 1, 0, 1, 0, 0), //+x
    mat3x3<f32>(0, 0, 1, 1, 0, 0, 0, 0, 0), //-y
    mat3x3<f32>(1, 0, 0, 0, 0, 1, 0, 1, 0), //+y
    mat3x3<f32>(1, 0, 0, 0, 1, 0, 0, 0, 0), //-z
    mat3x3<f32>(0, 1, 0, 1, 0, 0, 0, 0, 1), //+z
);

const M2: array<mat3x2<f32>, 3> = array<mat3x2<f32>, 3>(
    mat3x2<f32>(0, 0, 1, 0, 0, 1), // x
    mat3x2<f32>(1, 0, 0, 0, 0, 1), // y
    mat3x2<f32>(1, 0, 0, 1, 0, 0), // z
);

const normals: array<vec3<f32>, 6> = array<vec3<f32>, 6>(
    vec3<f32>(-1, 0, 0), //-x
    vec3<f32>(1, 0, 0),  //+x
    vec3<f32>(0, -1, 0), //-y
    vec3<f32>(0, 1, 0),  //+y
    vec3<f32>(0, 0, -1), //-z
    vec3<f32>(0, 0, 1),  //+z
);

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let corner = vec2<f32>(
        f32(in.vertexid & 1),
        f32((in.vertexid >> 1) & 1),
    );
    let index = in.cube_position.w;
    let scaled = M[index] * vec3<f32>(corner, 1) * (vec3<f32>(in.cube_size) / 2);
    let position = vec3<f32>(in.cube_position.xyz) + scaled;
    let tex_coords = M2[index / 2] * scaled;
    out.tex_coords = vec2<f32>(tex_coords.x, 1.0 - tex_coords.y);
    if index <= 1 {
        out.tex_coords = 1 - out.tex_coords.yx;
    }

    out.normal = normals[index];
    out.clip_position = mvp * vec4<f32>(position, 1);
    out.frag_position = position;
    out.square_index = index;
    out.tex_index = in.tex_index;
    out.color = in.color;
    return out;
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) frag_position: vec3<f32>,
    @location(3) @interpolate(flat) square_index: i32,
    @location(4) @interpolate(flat) tex_index: u32,
    @location(5) @interpolate(flat) color: u32,
}

@group(0) @binding(0)
var tex: texture_2d_array<f32>;
@group(0) @binding(1)
var samp: sampler;
 
const gamma = 1.5; // 1 - 3
const specular_strength = 0.5; // 0 - 1
const ambient = vec3<f32>(0.2); // 0 - 1

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_position = vec3<f32>(0.0, 300.0, 0.0);
    let light_color = vec3<f32>(1.0, 1.0, 1.0);
    let light_dir = normalize(light_position - in.frag_position);
    let diff = pow(max(dot(in.normal, light_dir), 0.0), gamma);
    let view_dir = normalize(-in.frag_position);
    let reflect_dir = reflect(-light_dir, in.normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0) * 0.0;
    let diffuse = ambient + diff * light_color + specular_strength * spec * light_color;
    let block_color = vec4f(vec4(
        (in.color >> 24) & 255,
        (in.color >> 16) & 255,
        (in.color >> 8) & 255,
        (in.color >> 0) & 255,
    )) / 255;
    let texture_color: vec4<f32> = textureSample(tex, samp, in.tex_coords, in.tex_index);
    return texture_color * vec4(diffuse, 1) * block_color;
}
 
 
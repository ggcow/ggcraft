@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // 8 vertices for a crosshair using triangle strip
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-.5, -.5),
        vec2<f32>(-.5, 1.5),
        vec2<f32>(1.5, -.5),
    );

    let pos = positions[vertex_index];
    return vec4<f32>(pos, 0.0, 1.0);
}

@group(0) @binding(0)
var<uniform> screen: vec2<u32>;

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let screen_f = vec2<f32>(screen);
    let center = screen_f * 0.5;

    let half_size = 10.0;   // demi-longueur de la croix en pixels

    // distances par rapport au centre
    let dx = frag_coord.x + 0.5 - center.x;
    let dy = frag_coord.y + 0.5 - center.y;

    // alpha lissé pour chaque barre
    let alpha_h = 1.0 - smoothstep(0., 1.0, abs(dy));
    let alpha_v = 1.0 - smoothstep(0., 1.0, abs(dx));

    // limiter la longueur des barres
    let alpha_h_final = alpha_h * smoothstep(half_size + 1.0, half_size, abs(dx));
    let alpha_v_final = alpha_v * smoothstep(half_size + 1.0, half_size, abs(dy));

    let alpha = max(alpha_h_final, alpha_v_final);
    return vec4<f32>(vec3<f32>(1.0), alpha);
}
struct Param {
  t: f32,
  s: f32
}
@group(0) @binding(0) var noise : texture_2d<f32>;
@group(0) @binding(1) var out : texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params : Param;
fn get_corner_val(theta: f32, d: vec2<f32>) -> f32 {
    return cos(theta * 8) * d.x + sin(theta * 8) * d.y;
}
fn quint_smooth(t: f32) -> f32 {
    return ((6 * t - 15) * t + 10) * t * t * t;
}
fn smoothlerp(t: f32, a: f32, b: f32) -> f32 {
    return a + quint_smooth(t) * (b - a);
}
fn bobulate(p: vec2<u32>, prime: u32) -> vec2<u32> {
    return p * prime / 256;
}
fn perlin_noise(location: vec2<f32>, time: f32, prime: u32) -> f32 {
    let bottom_left = vec2<u32>(location);
    let d = location - vec2<f32>(bottom_left);
    let bl_val = get_corner_val(textureLoad(noise, bobulate(bottom_left, prime), 0).x * 3.14 + time, d);
    let tl_val = get_corner_val(textureLoad(noise, bobulate(bottom_left + vec2(0, 1), prime), 0).x * 3.14 + time, vec2(d.x, d.y - 1.0));
    let br_val = get_corner_val(textureLoad(noise, bobulate(bottom_left + vec2(1, 0), prime), 0).x * 3.14 + time, vec2(d.x - 1.0, d.y));
    let tr_val = get_corner_val(textureLoad(noise, bobulate(bottom_left + vec2(1, 1), prime), 0).x * 3.14 + time, vec2(d.x - 1.0, d.y - 1.0));
    let l_val = smoothlerp(d.y, bl_val, tl_val);
    let r_val = smoothlerp(d.y, br_val, tr_val);
    let fin_val = smoothlerp(d.x, l_val, r_val);
    return fin_val;
}
@compute @workgroup_size(1)
fn draw(@builtin(global_invocation_id) id: vec3<u32>) {
    let noise_location = vec2<f32>(id.xy) / vec2(params.s);
    let val = perlin_noise(noise_location * 1, params.t, 1u) + perlin_noise(noise_location * 2, params.t, 353u) / 2 + perlin_noise(noise_location * 4, params.t, 367u) / 4 + perlin_noise(noise_location * 8, params.t, 431u) / 8;

    textureStore(out, id.xy, vec4(val) + 0.5);
}

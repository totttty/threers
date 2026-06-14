// DigitalGlitch — port of three.js DigitalGlitch shader (r165).
// Snow is precomputed in JS (setGlitchSnow) for GLSL sin() parity.

fn postfx_glitch(uv: vec2<f32>, res: vec2<f32>, base: vec4<f32>, fc: vec2<f32>) -> vec4<f32> {
    if (u.params.z >= 1.0) {
        return vec4<f32>(postfx_write_rgb(base.rgb), base.a);
    }
    var p = uv;
    let seed = u.params.y;
    let amount = u.params2.x;
    let angle = u.params2.y;
    let distortion_x = u.params2.z;
    let distortion_y = u.params2.w;
    let seed_x = u.params3.x;
    let seed_y = u.params3.y;
    let col_s = u.params3.z;
    let disp = textureSampleLevel(normal_tex, depth_sampler, p * seed * seed, 0.0).r;
    if (p.y < distortion_x + col_s && p.y > distortion_x - col_s * seed) {
        if (seed_x > 0.0) {
            p.y = 1.0 - (p.y + distortion_y);
        } else {
            p.y = distortion_y;
        }
    }
    if (p.x < distortion_y + col_s && p.x > distortion_y - col_s * seed) {
        if (seed_y > 0.0) {
            p.x = distortion_x;
        } else {
            p.x = 1.0 - (p.x + distortion_x);
        }
    }
    p.x = p.x + disp * seed_x * (seed / 5.0);
    p.y = p.y + disp * seed_y * (seed / 5.0);
    let offset = amount * vec2<f32>(cos(angle), sin(angle));
    let cr = textureSampleLevel(input_tex, input_sampler, p + offset, 0.0).r;
    let cga = textureSampleLevel(input_tex, input_sampler, p, 0.0);
    let cb = textureSampleLevel(input_tex, input_sampler, p - offset, 0.0).b;
    var col = vec4<f32>(cr, cga.g, cb, cga.a);
    let snow = textureSampleLevel(noise_tex, noise_sampler, uv, 0.0).r;
    col = col + vec4<f32>(snow, snow, snow, snow);
    return vec4<f32>(postfx_write_rgb(col.rgb), col.a);
}

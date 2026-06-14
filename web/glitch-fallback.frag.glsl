#version 300 es
precision highp float;
uniform int byp;
uniform sampler2D tDiffuse;
uniform sampler2D tDisp;
uniform float amount;
uniform float angle;
uniform float seed;
uniform float seed_x;
uniform float seed_y;
uniform float distortion_x;
uniform float distortion_y;
uniform float col_s;
in vec2 vUv;
out vec4 outColor;

void main() {
    if (byp < 1) {
        vec2 p = vUv;
        float disp = texture(tDisp, p * seed * seed).r;
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
        p.x += disp * seed_x * (seed / 5.0);
        p.y += disp * seed_y * (seed / 5.0);
        vec2 offset = amount * vec2(cos(angle), sin(angle));
        vec4 cr = texture(tDiffuse, p + offset);
        vec4 cga = texture(tDiffuse, p);
        vec4 cb = texture(tDiffuse, p - offset);
        outColor = vec4(cr.r, cga.g, cb.b, cga.a);
    } else {
        outColor = texture(tDiffuse, vUv);
    }
}

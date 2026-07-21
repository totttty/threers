//! Headless cube → browser WebM via pure-Rust VP9 (`native-codec`, no ffmpeg).
//!
//! ```text
//! cargo run --release --example export_webm_native --features native-codec
//! ```

use std::f32::consts::TAU;
use std::path::PathBuf;

use threers::{
    encode_animation_rgba, AmbientLight, AnimationEncodeOptions, BoxGeometry, BrowserCodec, Color,
    DirectionalLight, Euler, HeadlessRenderer, Mesh, Object3D, PerspectiveCamera, Scene,
    StandardMaterial, Vector3,
};

fn main() {
    let out = {
        let mut args = std::env::args().skip(1);
        let mut path = PathBuf::from("./out/cube.native.webm");
        while let Some(arg) = args.next() {
            if arg == "--out" {
                path = PathBuf::from(args.next().expect("--out needs a path"));
            }
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        path
    };

    let (w, h) = (320u32, 240u32); // multiples of 8 for VP9
    let mut hr = HeadlessRenderer::builder()
        .size(w, h)
        .build()
        .expect("headless renderer");
    let (rw, rh) = hr.render_size();

    let mut scene = Scene::new();
    scene.background = Color::new(0.05, 0.06, 0.09);
    scene.add_light(AmbientLight::new(Color::WHITE, 0.3));
    scene.add_light(
        DirectionalLight::new(Color::WHITE, 2.2)
            .with_direction(Vector3::new(-0.4, -0.8, -0.5).normalize()),
    );
    let mut mat = StandardMaterial::new(Color::new(0.20, 0.62, 0.95));
    mat.metalness = 0.15;
    mat.roughness = 0.35;
    let cube = scene.add(Object3D::mesh(Mesh::new(
        BoxGeometry::new(1.6, 1.6, 1.6),
        mat.into(),
    )));
    let mut cam = PerspectiveCamera::new(50.0, w as f32 / h as f32, 0.1, 100.0);
    cam.position = Vector3::new(0.0, 1.4, 4.2);
    cam.look_at(Vector3::ZERO);

    let frames = 30usize;
    let mut collected = Vec::with_capacity(frames);
    for f in 0..frames {
        let angle = f as f32 / frames as f32 * TAU;
        if let Some(o) = scene.get_mut(cube) {
            o.quaternion = Euler::new(0.5, angle, 0.0).to_quaternion();
        }
        collected.push(hr.render_to_rgba(&mut scene, &cam));
    }

    let opts = AnimationEncodeOptions {
        width: rw,
        height: rh,
        fps: 15,
        codec: BrowserCodec::Webm,
        transparent: false,
        gif_colors: 256,
    };
    let bytes = encode_animation_rgba(&opts, collected).expect("encode webm");
    std::fs::write(&out, &bytes).expect("write");
    println!("wrote {} ({} bytes)", out.display(), bytes.len());
}

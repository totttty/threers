use super::Texture;

/// Convenience constructor for textures populated with raw bytes (no decoder).
/// Mirrors three.js's `DataTexture`.
pub struct DataTexture;

impl DataTexture {
    pub fn new(width: u32, height: u32, format: super::TextureFormat, data: Vec<u8>) -> Texture {
        let mut t = Texture::new(width, height, format, data);
        // DataTextures default to no flip — matches three.js.
        t.flip_y = false;
        t
    }
}

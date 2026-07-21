#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color {
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
    };

    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    /// Build a color from a packed 0xRRGGBB hex value, three.js style.
    /// three.js treats hex/CSS colors as sRGB and decodes them to linear for
    /// shader work, so we do the same — otherwise PBR output diverges by the
    /// linear→sRGB curve on every color literal.
    pub fn from_hex(hex: u32) -> Self {
        fn srgb_to_linear(s: f32) -> f32 {
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        }
        Self {
            r: srgb_to_linear(((hex >> 16) & 0xff) as f32 / 255.0),
            g: srgb_to_linear(((hex >> 8) & 0xff) as f32 / 255.0),
            b: srgb_to_linear((hex & 0xff) as f32 / 255.0),
        }
    }

    pub fn to_array(&self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

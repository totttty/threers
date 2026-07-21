/// Channel bitmask. Mirrors three.js's `Layers`: each Object3D/Camera has one,
/// and a mesh is visible to a camera iff `camera.layers.test(&mesh.layers)`.
///
/// Defaults to channel 0 enabled (matches three.js).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layers {
    pub mask: u32,
}

impl Default for Layers {
    fn default() -> Self {
        Self { mask: 1 }
    }
}

impl Layers {
    pub const fn new() -> Self {
        Self { mask: 1 }
    }

    pub const fn with_mask(mask: u32) -> Self {
        Self { mask }
    }

    pub fn set(&mut self, channel: u8) -> &mut Self {
        self.mask = 1u32 << (channel & 31);
        self
    }

    pub fn enable(&mut self, channel: u8) -> &mut Self {
        self.mask |= 1u32 << (channel & 31);
        self
    }

    pub fn enable_all(&mut self) -> &mut Self {
        self.mask = u32::MAX;
        self
    }

    pub fn disable(&mut self, channel: u8) -> &mut Self {
        self.mask &= !(1u32 << (channel & 31));
        self
    }

    pub fn disable_all(&mut self) -> &mut Self {
        self.mask = 0;
        self
    }

    pub fn toggle(&mut self, channel: u8) -> &mut Self {
        self.mask ^= 1u32 << (channel & 31);
        self
    }

    pub fn test(&self, other: &Self) -> bool {
        (self.mask & other.mask) != 0
    }

    pub fn is_enabled(&self, channel: u8) -> bool {
        (self.mask & (1u32 << (channel & 31))) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_channel_zero_enabled() {
        let l = Layers::default();
        assert!(l.is_enabled(0));
        assert!(!l.is_enabled(1));
    }

    #[test]
    fn enable_disable_toggle() {
        let mut l = Layers::default();
        l.enable(5);
        assert!(l.is_enabled(5));
        l.disable(5);
        assert!(!l.is_enabled(5));
        l.toggle(7);
        assert!(l.is_enabled(7));
        l.toggle(7);
        assert!(!l.is_enabled(7));
    }

    #[test]
    fn test_intersection() {
        let mut a = Layers::default();
        let mut b = Layers::with_mask(0);
        b.enable(0);
        assert!(a.test(&b));
        a.set(2);
        assert!(!a.test(&b));
        b.enable(2);
        assert!(a.test(&b));
    }
}

use super::Audio;

/// Spatial audio source. Inherits all `Audio` parameters plus a distance model.
/// Use as `ObjectKind`-attached data (TODO: wire up an Audio variant when the
/// playback backend lands).
#[derive(Debug, Clone)]
pub struct PositionalAudio {
    pub audio: Audio,
    pub ref_distance: f32,
    pub max_distance: f32,
    pub rolloff_factor: f32,
    /// Outer cone angle in radians (or `f32::INFINITY` for omni).
    pub cone_outer_angle: f32,
    pub cone_inner_angle: f32,
    pub cone_outer_gain: f32,
}

impl Default for PositionalAudio {
    fn default() -> Self {
        Self {
            audio: Audio::default(),
            ref_distance: 1.0,
            max_distance: 10_000.0,
            rolloff_factor: 1.0,
            cone_outer_angle: f32::INFINITY,
            cone_inner_angle: f32::INFINITY,
            cone_outer_gain: 0.0,
        }
    }
}

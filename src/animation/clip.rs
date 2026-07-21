use super::KeyframeTrack;

/// Named collection of tracks plus a total duration.
#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub tracks: Vec<KeyframeTrack>,
}

impl AnimationClip {
    pub fn new(name: impl Into<String>, duration: f32, tracks: Vec<KeyframeTrack>) -> Self {
        Self {
            name: name.into(),
            duration,
            tracks,
        }
    }

    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            duration: 0.0,
            tracks: Vec::new(),
        }
    }
}

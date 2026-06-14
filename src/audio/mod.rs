//! Audio data types. Modeled on three.js's `Audio`/`AudioListener`/
//! `PositionalAudio`/`AudioAnalyser`. The actual playback backend is left to
//! the user (cpal natively, Web Audio in wasm) — this crate only owns the
//! data shape so other subsystems (scene graph, animation) can reference it.

mod listener;
mod source;
mod positional;
mod analyser;
mod backend;

pub use listener::AudioListener;
pub use source::Audio;
pub use positional::PositionalAudio;
pub use analyser::AudioAnalyser;
pub use backend::{AudioBackend, NoopBackend};

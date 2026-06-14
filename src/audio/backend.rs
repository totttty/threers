use super::Audio;

/// Audio playback backend. The default `NoopBackend` does nothing; users can
/// plug in cpal/rodio natively or web_sys::AudioContext on wasm by
/// implementing this trait themselves.
pub trait AudioBackend: Send + Sync {
    fn play(&mut self, source: &Audio);
    fn stop(&mut self, source: &Audio);
    fn set_master_volume(&mut self, volume: f32);
}

#[derive(Debug, Default)]
pub struct NoopBackend;

impl AudioBackend for NoopBackend {
    fn play(&mut self, _source: &Audio) {}
    fn stop(&mut self, _source: &Audio) {}
    fn set_master_volume(&mut self, _volume: f32) {}
}

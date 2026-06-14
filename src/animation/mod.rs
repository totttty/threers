//! Keyframe animation. Mirrors three.js's `AnimationClip` / `AnimationMixer`
//! / `AnimationAction` / `KeyframeTrack` model.
//!
//! A `Clip` is a named set of `Track`s. Each `Track` targets a node's
//! property by an `ObjectId` + `TrackTarget` enum (position/quaternion/scale/etc.).
//! A `Mixer` advances time and applies the sampled values back to its `Scene`.
//! `Action`s are play/pause handles over individual clips.

mod interpolant;
mod track;
mod clip;
mod mixer;
mod action;

pub use interpolant::Interpolation;
pub use track::{KeyframeTrack, TrackTarget};
pub use clip::AnimationClip;
pub use mixer::AnimationMixer;
pub use action::AnimationAction;

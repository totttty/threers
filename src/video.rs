//! Native video export (behind the `video` feature). Renders a frame sequence
//! and encodes it — by default with the system `ffmpeg`. When both `video` and
//! `native-codec` are enabled, [`VideoCodec::Gif`] streams frames through the
//! pure-Rust [`crate::GifWriter`] (no ffmpeg); other codecs still require ffmpeg.
//!
//! Engine-agnostic: you supply a closure that returns the RGBA bytes for a frame
//! index (e.g. from [`HeadlessRenderer::render_to_rgba`](crate::HeadlessRenderer)).
//!
//! ```no_run
//! use threers::{HeadlessRenderer, VideoOptions, VideoCodec, export_video, Scene, PerspectiveCamera};
//! let (w, h) = (1280, 720);
//! let mut hr = HeadlessRenderer::builder().size(w, h).build().unwrap();
//! let mut scene = Scene::new();
//! let cam = PerspectiveCamera::new(50.0, w as f32 / h as f32, 0.1, 1000.0);
//! let opts = VideoOptions::new("out.mp4").fps(30).codec(VideoCodec::H264);
//! export_video(w, h, 90, &opts, |_frame| {
//!     // ...advance the scene for this frame...
//!     hr.render_to_rgba(&mut scene, &cam)
//! }).unwrap();
//! ```

use std::io::Write;
use std::process::{Command, Stdio};

/// Output video codec / container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoCodec {
    /// H.264 (`libx264`) → `.mp4`. Widely compatible.
    H264,
    /// H.265 / HEVC (`libx265`) → `.mp4` (`hvc1` tag).
    Hevc,
    /// Hardware HEVC on Apple platforms (`hevc_videotoolbox`) — fast.
    HevcVideoToolbox,
    /// VP9 (`libvpx-vp9`) → `.webm`. Set `transparent` for an alpha channel.
    Vp9,
    /// Animated GIF. With `native-codec`, encoded in-process; otherwise ffmpeg.
    Gif,
}

impl VideoCodec {
    fn is_crf_based(self) -> bool {
        matches!(self, VideoCodec::H264 | VideoCodec::Hevc | VideoCodec::Vp9)
    }
}

/// Encoding quality — a target bitrate or a constant-rate-factor.
#[derive(Clone, Debug)]
pub enum VideoQuality {
    /// Codec default.
    Default,
    /// Target bitrate string, e.g. `"12M"`.
    Bitrate(String),
    /// Constant rate factor (lower = higher quality; ~18–28 typical). Ignored by
    /// hardware/GIF codecs, which fall back to bitrate/default.
    Crf(u32),
}

/// Options for [`export_video`].
#[derive(Clone, Debug)]
pub struct VideoOptions {
    /// Output file path (extension should match the codec/container).
    pub output: String,
    /// Frames per second.
    pub fps: u32,
    /// Codec / container.
    pub codec: VideoCodec,
    /// Quality target.
    pub quality: VideoQuality,
    /// Preserve the alpha channel where the codec supports it (VP9 → yuva420p;
    /// GIF → transparent index).
    pub transparent: bool,
    /// Extra raw ffmpeg args appended before the output path (escape hatch).
    pub extra_args: Vec<String>,
    /// GIF palette size when using the native encoder (`2..=256`, default 256).
    pub gif_colors: u16,
}

impl VideoOptions {
    /// New options for `output` with sensible defaults (30 fps, H.264).
    pub fn new(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            fps: 30,
            codec: VideoCodec::H264,
            quality: VideoQuality::Default,
            transparent: false,
            extra_args: Vec::new(),
            gif_colors: 256,
        }
    }
    /// Frames per second (minimum 1).
    pub fn fps(mut self, fps: u32) -> Self {
        self.fps = fps.max(1);
        self
    }
    /// Output codec / container.
    pub fn codec(mut self, codec: VideoCodec) -> Self {
        self.codec = codec;
        self
    }
    /// Target bitrate string for ffmpeg (e.g. `"12M"`).
    pub fn bitrate(mut self, bitrate: impl Into<String>) -> Self {
        self.quality = VideoQuality::Bitrate(bitrate.into());
        self
    }
    /// Constant rate factor (lower = higher quality).
    pub fn crf(mut self, crf: u32) -> Self {
        self.quality = VideoQuality::Crf(crf);
        self
    }
    /// Request an alpha channel where the codec supports it.
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }
    /// Extra ffmpeg CLI args inserted before the output path.
    pub fn extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }
    /// Native GIF palette size (`2..=256`).
    pub fn gif_colors(mut self, n: u16) -> Self {
        self.gif_colors = n.clamp(2, 256);
        self
    }
}

/// Errors from [`export_video`].
#[derive(Debug)]
pub enum VideoError {
    /// `ffmpeg` could not be launched (is it installed / on `PATH`?).
    Spawn(std::io::Error),
    /// Failed to write a frame to ffmpeg's stdin.
    Write(std::io::Error),
    /// Failed to write the output file (native GIF path).
    Io(std::io::Error),
    /// A frame closure returned the wrong number of bytes (`expected`, `got`).
    FrameSize {
        frame: usize,
        expected: usize,
        got: usize,
    },
    /// ffmpeg exited with a non-zero status.
    Ffmpeg(Option<i32>),
    /// `frames` was zero.
    NoFrames,
}

impl std::fmt::Display for VideoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VideoError::Spawn(e) => {
                write!(f, "failed to launch ffmpeg (installed and on PATH?): {e}")
            }
            VideoError::Write(e) => write!(f, "failed to write a frame to ffmpeg: {e}"),
            VideoError::Io(e) => write!(f, "failed to write output: {e}"),
            VideoError::FrameSize {
                frame,
                expected,
                got,
            } => write!(
                f,
                "frame {frame} returned {got} bytes, expected {expected} (width*height*4)"
            ),
            VideoError::Ffmpeg(code) => write!(f, "ffmpeg exited with status {code:?}"),
            VideoError::NoFrames => write!(f, "frame count was zero"),
        }
    }
}
impl std::error::Error for VideoError {}

/// Encode `frames` frames of `width × height` RGBA into `options.output`.
///
/// `frame` is called with each index `0..frames` and must return exactly
/// `width * height * 4` bytes (tightly-packed RGBA8, top-left origin).
pub fn export_video<F>(
    width: u32,
    height: u32,
    frames: usize,
    options: &VideoOptions,
    mut frame: F,
) -> Result<(), VideoError>
where
    F: FnMut(usize) -> Vec<u8>,
{
    if frames == 0 {
        return Err(VideoError::NoFrames);
    }
    let expected = (width as usize) * (height as usize) * 4;

    #[cfg(feature = "native-codec")]
    if options.codec == VideoCodec::Gif {
        return export_gif_native(width, height, frames, options, expected, &mut frame);
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y")
        .args(["-f", "rawvideo"])
        .args(["-pixel_format", "rgba"])
        .args(["-video_size", &format!("{width}x{height}")])
        .args(["-framerate", &options.fps.to_string()])
        .args(["-i", "-"]);
    append_codec_args(&mut cmd, options);
    for a in &options.extra_args {
        cmd.arg(a);
    }
    cmd.arg(&options.output);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    let mut child = cmd.spawn().map_err(VideoError::Spawn)?;
    {
        let mut stdin = child.stdin.take().expect("ffmpeg stdin");
        for i in 0..frames {
            let buf = frame(i);
            if buf.len() != expected {
                let _ = child.kill();
                return Err(VideoError::FrameSize {
                    frame: i,
                    expected,
                    got: buf.len(),
                });
            }
            stdin.write_all(&buf).map_err(VideoError::Write)?;
        }
    }
    let status = child.wait().map_err(VideoError::Spawn)?;
    if !status.success() {
        return Err(VideoError::Ffmpeg(status.code()));
    }
    Ok(())
}

#[cfg(feature = "native-codec")]
fn export_gif_native<F>(
    width: u32,
    height: u32,
    frames: usize,
    options: &VideoOptions,
    expected: usize,
    frame: &mut F,
) -> Result<(), VideoError>
where
    F: FnMut(usize) -> Vec<u8>,
{
    use crate::codec::gif::{GifOptions, GifWriter, PaletteMode};
    use std::fs::File;

    let delay_den = options.fps.max(1) as u16;
    let opts = GifOptions::default()
        .colors(options.gif_colors)
        .transparency(options.transparent)
        .dither(true)
        .diff_rects(true)
        .palette_mode(PaletteMode::Local);

    let file = File::create(&options.output).map_err(VideoError::Io)?;
    let mut writer = GifWriter::new(file, width, height, 0, opts).map_err(VideoError::Io)?;
    for i in 0..frames {
        let buf = frame(i);
        if buf.len() != expected {
            return Err(VideoError::FrameSize {
                frame: i,
                expected,
                got: buf.len(),
            });
        }
        writer
            .write_frame(&buf, 1, delay_den)
            .map_err(VideoError::Io)?;
    }
    writer.finish().map_err(VideoError::Io)?;
    Ok(())
}

fn append_codec_args(cmd: &mut Command, opts: &VideoOptions) {
    match opts.codec {
        VideoCodec::H264 => {
            cmd.args(["-c:v", "libx264", "-pix_fmt", "yuv420p"]);
        }
        VideoCodec::Hevc => {
            cmd.args(["-c:v", "libx265", "-pix_fmt", "yuv420p", "-tag:v", "hvc1"]);
        }
        VideoCodec::HevcVideoToolbox => {
            cmd.args([
                "-c:v",
                "hevc_videotoolbox",
                "-tag:v",
                "hvc1",
                "-pix_fmt",
                "yuv420p",
            ]);
        }
        VideoCodec::Vp9 => {
            let pix = if opts.transparent {
                "yuva420p"
            } else {
                "yuv420p"
            };
            cmd.args(["-c:v", "libvpx-vp9", "-pix_fmt", pix]);
        }
        VideoCodec::Gif => {
            cmd.args([
                "-vf",
                "split[s0][s1];[s0]palettegen=stats_mode=diff[p];[s1][p]paletteuse",
            ]);
        }
    }
    match &opts.quality {
        VideoQuality::Default => {}
        VideoQuality::Bitrate(b) => {
            cmd.args(["-b:v", b]);
        }
        VideoQuality::Crf(crf) => {
            if opts.codec.is_crf_based() {
                cmd.args(["-crf", &crf.to_string()]);
            }
        }
    }
}

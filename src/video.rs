//! Native video export (behind the `video` feature). Renders a frame sequence
//! and encodes it — by default with the system `ffmpeg`. When both `video` and
//! `native-codec` are enabled, [`VideoCodec::Gif`] and [`VideoCodec::Apng`] are
//! encoded in-process (no ffmpeg); other codecs still require ffmpeg.
//!
//! For browser / wasm downloads (GIF, APNG, WebM) use
//! [`crate::encode_animation_rgba`] with [`crate::BrowserCodec`] instead — that
//! API returns bytes and never shells out.
//!
//! Engine-agnostic: you supply a closure that returns the RGBA bytes for a frame
//! index (e.g. from [`HeadlessRenderer::render_to_rgba`](crate::HeadlessRenderer)).
//!
//! Trace progress with [`VideoExporter`] events (mirrors the JS API):
//!
//! ```no_run
//! use threers::{VideoCodec, VideoExporter, VideoExportEvent};
//! # fn render(_i: usize) -> Vec<u8> { vec![0; 4] }
//! VideoExporter::new("out.gif")
//!     .size(64, 64)
//!     .frames(8)
//!     .fps(10)
//!     .codec(VideoCodec::Gif)
//!     .on_event(|ev| match ev {
//!         VideoExportEvent::Progress(p) => eprintln!("{}", p.message),
//!         VideoExportEvent::Complete { output, .. } => eprintln!("wrote {output}"),
//!         _ => {}
//!     })
//!     .export(render)
//!     .unwrap();
//! ```
//!
//! See examples `export_h264`, `export_hevc`, `export_vp9`, `export_gif`, …
//! and the web demo `web/examples/export-video.html`.
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
    /// Animated PNG. With `native-codec`, encoded in-process; otherwise ffmpeg.
    Apng,
}

impl VideoCodec {
    fn is_crf_based(self) -> bool {
        matches!(self, VideoCodec::H264 | VideoCodec::Hevc | VideoCodec::Vp9)
    }

    /// Short label for progress messages (`"gif"`, `"h264"`, …).
    pub fn label(self) -> &'static str {
        match self {
            VideoCodec::H264 => "h264",
            VideoCodec::Hevc => "hevc",
            VideoCodec::HevcVideoToolbox => "hevc-vt",
            VideoCodec::Vp9 => "webm",
            VideoCodec::Gif => "gif",
            VideoCodec::Apng => "apng",
        }
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
    /// Preserve the alpha channel where the codec supports it (VP9 → yuva420p,
    /// GIF → transparent index, APNG → RGBA).
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

/// Progress phase — mirrors the JS `VideoExportPhase`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoExportPhase {
    Capture,
    Encode,
    Done,
}

impl VideoExportPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Capture => "capture",
            Self::Encode => "encode",
            Self::Done => "done",
        }
    }
}

/// Progress tick — mirrors the JS `VideoExportProgress`.
#[derive(Clone, Debug)]
pub struct VideoExportProgress {
    pub phase: VideoExportPhase,
    /// 1-based capture index, or encode cursor.
    pub frame: u32,
    pub frames: u32,
    /// Overall 0..=1 estimate.
    pub ratio: f32,
    pub codec: VideoCodec,
    /// Ready-to-display status line from [`format_video_progress`].
    pub message: String,
}

/// Lifecycle / progress events — mirrors the JS `VideoExportEvent` names.
#[derive(Clone, Debug)]
pub enum VideoExportEvent {
    Start {
        phase: VideoExportPhase,
        frames: u32,
        codec: VideoCodec,
    },
    Progress(VideoExportProgress),
    Capture(VideoExportProgress),
    Encode(VideoExportProgress),
    Complete {
        output: String,
        frames: u32,
        codec: VideoCodec,
    },
    Error {
        message: String,
    },
}

/// Ready-to-display progress line (e.g. `"Rendering 12/30 (40%)"`).
pub fn format_video_progress(info: &VideoExportProgress) -> String {
    let pct = (info.ratio.clamp(0.0, 1.0) * 100.0).round() as i32;
    match info.phase {
        VideoExportPhase::Capture => {
            format!("Rendering {}/{} ({pct}%)", info.frame, info.frames)
        }
        VideoExportPhase::Encode => {
            format!(
                "Encoding {}… ({pct}%)",
                info.codec.label().to_ascii_uppercase()
            )
        }
        VideoExportPhase::Done => format!("Done ({} frames)", info.frames),
    }
}

fn make_progress(
    phase: VideoExportPhase,
    frame: u32,
    frames: u32,
    ratio: f32,
    codec: VideoCodec,
) -> VideoExportProgress {
    let mut info = VideoExportProgress {
        phase,
        frame,
        frames,
        ratio: ratio.clamp(0.0, 1.0),
        codec,
        message: String::new(),
    };
    info.message = format_video_progress(&info);
    info
}

/// Collects progress / lifecycle listeners for [`VideoExporter`].
#[derive(Default)]
struct VideoExportTracer {
    on_progress: Option<Box<dyn FnMut(&VideoExportProgress) + Send>>,
    on_event: Option<Box<dyn FnMut(&VideoExportEvent) + Send>>,
}

impl VideoExportTracer {
    fn emit_event(&mut self, event: VideoExportEvent) {
        if let VideoExportEvent::Progress(ref p)
        | VideoExportEvent::Capture(ref p)
        | VideoExportEvent::Encode(ref p) = event
        {
            if let Some(cb) = self.on_progress.as_mut() {
                cb(p);
            }
        }
        if let Some(cb) = self.on_event.as_mut() {
            cb(&event);
        }
    }

    fn emit_progress(&mut self, info: VideoExportProgress) {
        match info.phase {
            VideoExportPhase::Capture => {
                self.emit_event(VideoExportEvent::Progress(info.clone()));
                self.emit_event(VideoExportEvent::Capture(info));
            }
            VideoExportPhase::Encode => {
                self.emit_event(VideoExportEvent::Progress(info.clone()));
                self.emit_event(VideoExportEvent::Encode(info));
            }
            VideoExportPhase::Done => {
                self.emit_event(VideoExportEvent::Progress(info));
            }
        }
    }
}

/// Fluent video exporter with JS-style progress events.
///
/// ```no_run
/// use threers::{VideoCodec, VideoExporter, VideoExportEvent};
/// # fn render(_: usize) -> Vec<u8> { vec![0; 8 * 8 * 4] }
/// VideoExporter::new("out.gif")
///     .size(8, 8)
///     .frames(4)
///     .fps(10)
///     .codec(VideoCodec::Gif)
///     .on_progress(|p| println!("{}", p.message))
///     .export(render)
///     .unwrap();
/// ```
pub struct VideoExporter {
    options: VideoOptions,
    width: u32,
    height: u32,
    frames: usize,
    tracer: VideoExportTracer,
}

impl VideoExporter {
    /// Start a fluent export targeting `output`.
    pub fn new(output: impl Into<String>) -> Self {
        Self {
            options: VideoOptions::new(output),
            width: 0,
            height: 0,
            frames: 0,
            tracer: VideoExportTracer::default(),
        }
    }

    /// Frame size in pixels.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Number of frames to capture / encode.
    pub fn frames(mut self, frames: usize) -> Self {
        self.frames = frames;
        self
    }

    pub fn fps(mut self, fps: u32) -> Self {
        self.options = self.options.fps(fps);
        self
    }

    pub fn codec(mut self, codec: VideoCodec) -> Self {
        self.options = self.options.codec(codec);
        self
    }

    pub fn bitrate(mut self, bitrate: impl Into<String>) -> Self {
        self.options = self.options.bitrate(bitrate);
        self
    }

    pub fn crf(mut self, crf: u32) -> Self {
        self.options = self.options.crf(crf);
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.options = self.options.transparent(transparent);
        self
    }

    pub fn gif_colors(mut self, n: u16) -> Self {
        self.options = self.options.gif_colors(n);
        self
    }

    pub fn extra_args(mut self, args: Vec<String>) -> Self {
        self.options = self.options.extra_args(args);
        self
    }

    /// Replace options wholesale (keeps size / frame count / listeners).
    pub fn options(mut self, options: VideoOptions) -> Self {
        self.options = options;
        self
    }

    /// Progress callback (JS `onProgress` / `progress` event).
    pub fn on_progress(
        mut self,
        cb: impl FnMut(&VideoExportProgress) + Send + 'static,
    ) -> Self {
        self.tracer.on_progress = Some(Box::new(cb));
        self
    }

    /// Lifecycle + progress callback (JS `addEventListener` / `.on(...)`).
    pub fn on_event(mut self, cb: impl FnMut(&VideoExportEvent) + Send + 'static) -> Self {
        self.tracer.on_event = Some(Box::new(cb));
        self
    }

    /// Capture frames via `frame(index)` and encode to the configured output.
    pub fn export<F>(mut self, frame: F) -> Result<(), VideoError>
    where
        F: FnMut(usize) -> Vec<u8>,
    {
        if self.width == 0 || self.height == 0 {
            return Err(VideoError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "VideoExporter: size(width, height) required",
            )));
        }
        export_video_traced(
            self.width,
            self.height,
            self.frames,
            &self.options,
            frame,
            &mut self.tracer,
        )
    }
}

/// Errors from [`export_video`].
#[derive(Debug)]
pub enum VideoError {
    /// `ffmpeg` could not be launched (is it installed / on `PATH`?).
    Spawn(std::io::Error),
    /// Failed to write a frame to ffmpeg's stdin.
    Write(std::io::Error),
    /// Failed to write the output file (native animation path).
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
///
/// Prefer [`VideoExporter`] when you want progress events.
pub fn export_video<F>(
    width: u32,
    height: u32,
    frames: usize,
    options: &VideoOptions,
    frame: F,
) -> Result<(), VideoError>
where
    F: FnMut(usize) -> Vec<u8>,
{
    let mut tracer = VideoExportTracer::default();
    export_video_traced(width, height, frames, options, frame, &mut tracer)
}

/// Like [`export_video`], but reports progress through `on_progress`.
pub fn export_video_with_progress<F, P>(
    width: u32,
    height: u32,
    frames: usize,
    options: &VideoOptions,
    frame: F,
    on_progress: P,
) -> Result<(), VideoError>
where
    F: FnMut(usize) -> Vec<u8>,
    P: FnMut(&VideoExportProgress) + Send + 'static,
{
    VideoExporter::new(options.output.clone())
        .size(width, height)
        .frames(frames)
        .options(options.clone())
        .on_progress(on_progress)
        .export(frame)
}

fn export_video_traced<F>(
    width: u32,
    height: u32,
    frames: usize,
    options: &VideoOptions,
    mut frame: F,
    tracer: &mut VideoExportTracer,
) -> Result<(), VideoError>
where
    F: FnMut(usize) -> Vec<u8>,
{
    if frames == 0 {
        let err = VideoError::NoFrames;
        tracer.emit_event(VideoExportEvent::Error {
            message: err.to_string(),
        });
        return Err(err);
    }
    let expected = (width as usize) * (height as usize) * 4;
    let frames_u = frames as u32;
    let codec = options.codec;

    let result = (|| {
        #[cfg(feature = "native-codec")]
        if matches!(options.codec, VideoCodec::Gif | VideoCodec::Apng) {
            return export_native_animation(width, height, frames, options, &mut frame, tracer);
        }

        tracer.emit_event(VideoExportEvent::Start {
            phase: VideoExportPhase::Capture,
            frames: frames_u,
            codec,
        });

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
                let done = (i + 1) as u32;
                tracer.emit_progress(make_progress(
                    VideoExportPhase::Capture,
                    done,
                    frames_u,
                    done as f32 / (frames_u as f32 + 1.0),
                    codec,
                ));
            }
        }
        let status = child.wait().map_err(VideoError::Spawn)?;
        if !status.success() {
            return Err(VideoError::Ffmpeg(status.code()));
        }
        tracer.emit_progress(make_progress(
            VideoExportPhase::Done,
            frames_u,
            frames_u,
            1.0,
            codec,
        ));
        Ok(())
    })();

    match &result {
        Ok(()) => tracer.emit_event(VideoExportEvent::Complete {
            output: options.output.clone(),
            frames: frames_u,
            codec,
        }),
        Err(e) => tracer.emit_event(VideoExportEvent::Error {
            message: e.to_string(),
        }),
    }
    result
}

#[cfg(feature = "native-codec")]
fn export_native_animation<F>(
    width: u32,
    height: u32,
    frames: usize,
    options: &VideoOptions,
    frame: &mut F,
    tracer: &mut VideoExportTracer,
) -> Result<(), VideoError>
where
    F: FnMut(usize) -> Vec<u8>,
{
    use crate::codec::animation::{
        encode_animation_rgba_with_progress, AnimationEncodeError, AnimationEncodeOptions,
        AnimationExportPhase, BrowserCodec,
    };

    let codec = match options.codec {
        VideoCodec::Gif => BrowserCodec::Gif,
        VideoCodec::Apng => BrowserCodec::Apng,
        _ => unreachable!("native animation export only handles GIF and APNG"),
    };
    let opts = AnimationEncodeOptions {
        width,
        height,
        fps: options.fps,
        codec,
        transparent: options.transparent,
        gif_colors: options.gif_colors,
    };
    let expected = (width as usize) * (height as usize) * 4;
    let frames_u = frames as u32;
    let vcodec = options.codec;

    tracer.emit_event(VideoExportEvent::Start {
        phase: VideoExportPhase::Capture,
        frames: frames_u,
        codec: vcodec,
    });

    let mut captured = Vec::with_capacity(frames);
    for i in 0..frames {
        let buf = frame(i);
        if buf.len() != expected {
            return Err(VideoError::FrameSize {
                frame: i,
                expected,
                got: buf.len(),
            });
        }
        captured.push(buf);
        let done = (i + 1) as u32;
        tracer.emit_progress(make_progress(
            VideoExportPhase::Capture,
            done,
            frames_u,
            done as f32 / (frames_u as f32 + 1.0),
            vcodec,
        ));
    }

    tracer.emit_event(VideoExportEvent::Start {
        phase: VideoExportPhase::Encode,
        frames: frames_u,
        codec: vcodec,
    });

    let bytes = encode_animation_rgba_with_progress(&opts, captured, |p| {
        let phase = match p.phase {
            AnimationExportPhase::Encode => VideoExportPhase::Encode,
            AnimationExportPhase::Done => VideoExportPhase::Done,
            AnimationExportPhase::Capture => VideoExportPhase::Capture,
        };
        tracer.emit_progress(make_progress(
            phase, p.frame, p.frames, p.ratio, vcodec,
        ));
    })
    .map_err(|error| match error {
        AnimationEncodeError::Empty => VideoError::NoFrames,
        AnimationEncodeError::FrameSize {
            frame,
            expected,
            got,
        } => VideoError::FrameSize {
            frame,
            expected,
            got,
        },
        AnimationEncodeError::Dimension => VideoError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid native animation dimensions or options",
        )),
        AnimationEncodeError::Io(message) => VideoError::Io(std::io::Error::other(message)),
    })?;
    std::fs::write(&options.output, bytes).map_err(VideoError::Io)?;
    tracer.emit_progress(make_progress(
        VideoExportPhase::Done,
        frames_u,
        frames_u,
        1.0,
        vcodec,
    ));
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
        VideoCodec::Apng => {
            cmd.args(["-plays", "0", "-f", "apng"]);
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

#[cfg(all(test, feature = "native-codec"))]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
        let mut out = vec![0u8; (w * h * 4) as usize];
        for px in out.chunks_exact_mut(4) {
            px.copy_from_slice(&[r, g, b, 255]);
        }
        out
    }

    #[test]
    fn video_exporter_emits_progress_events() {
        let dir = std::env::temp_dir().join("threers-video-events");
        let _ = std::fs::create_dir_all(&dir);
        let out = dir.join("cube.gif");
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let ev2 = events.clone();
        let prog = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let p2 = prog.clone();

        VideoExporter::new(out.to_string_lossy().into_owned())
            .size(8, 8)
            .frames(4)
            .fps(10)
            .codec(VideoCodec::Gif)
            .gif_colors(32)
            .on_progress(move |p| p2.lock().unwrap().push(p.message.clone()))
            .on_event(move |e| {
                let label = match e {
                    VideoExportEvent::Start { phase, .. } => format!("start:{}", phase.as_str()),
                    VideoExportEvent::Progress(p) => format!("progress:{}", p.phase.as_str()),
                    VideoExportEvent::Capture(_) => "capture".into(),
                    VideoExportEvent::Encode(_) => "encode".into(),
                    VideoExportEvent::Complete { .. } => "complete".into(),
                    VideoExportEvent::Error { message } => format!("error:{message}"),
                };
                ev2.lock().unwrap().push(label);
            })
            .export(|i| {
                let c = ((i * 40) % 255) as u8;
                solid(8, 8, c, 80, 200)
            })
            .expect("export");

        let events = events.lock().unwrap().clone();
        assert!(events.iter().any(|e| e.starts_with("start:")), "{events:?}");
        assert!(events.iter().any(|e| e == "capture"), "{events:?}");
        assert!(events.iter().any(|e| e == "encode"), "{events:?}");
        assert!(events.iter().any(|e| e == "complete"), "{events:?}");
        let prog = prog.lock().unwrap().clone();
        assert!(!prog.is_empty(), "expected progress messages");
        assert!(out.is_file());
        let bytes = std::fs::read(&out).unwrap();
        assert_eq!(&bytes[..3], b"GIF");
    }
}

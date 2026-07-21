//! Minimal WebM (Matroska / EBML) muxer for a single video track (pure Rust,
//! wasm-safe).
//!
//! WebM is the subset of Matroska that browsers play: an EBML document whose
//! `Segment` holds `Info`, `Tracks`, and one or more `Cluster`s of frames. This
//! muxer emits exactly that for a single VP8 / VP9 / AV1 video track — enough for
//! ffmpeg and every WebM-capable browser to open the result.
//!
//! Optional **alpha** is carried the WebM way: `AlphaMode = 1` on the track and a
//! second VP9 bitstream in `BlockAdditional` (`BlockAddID = 1`) inside a
//! `BlockGroup` (see Matroska codec mapping for `V_VP9`).
//!
//! Everything is built bottom-up into `Vec<u8>` so each element's size is known
//! before its parent is written (no streaming "unknown size" VINTs needed). The
//! codec bitstream itself comes from the caller — pair this with a native VP9
//! encoder (see [`crate::codec::vp9`]) for a fully dependency-free `.webm`.

// ---- EBML element IDs (canonical, marker bits included) ----
const ID_EBML: u32 = 0x1A45_DFA3;
const ID_EBML_VERSION: u32 = 0x4286;
const ID_EBML_READ_VERSION: u32 = 0x42F7;
const ID_EBML_MAX_ID_LENGTH: u32 = 0x42F2;
const ID_EBML_MAX_SIZE_LENGTH: u32 = 0x42F3;
const ID_DOC_TYPE: u32 = 0x4282;
const ID_DOC_TYPE_VERSION: u32 = 0x4287;
const ID_DOC_TYPE_READ_VERSION: u32 = 0x4285;

const ID_SEGMENT: u32 = 0x1853_8067;
const ID_INFO: u32 = 0x1549_A966;
const ID_TIMECODE_SCALE: u32 = 0x2AD7_B1;
const ID_DURATION: u32 = 0x4489;
const ID_MUXING_APP: u32 = 0x4D80;
const ID_WRITING_APP: u32 = 0x5741;

const ID_TRACKS: u32 = 0x1654_AE6B;
const ID_TRACK_ENTRY: u32 = 0xAE;
const ID_TRACK_NUMBER: u32 = 0xD7;
const ID_TRACK_UID: u32 = 0x73C5;
const ID_TRACK_TYPE: u32 = 0x83;
const ID_FLAG_LACING: u32 = 0x9C;
const ID_CODEC_ID: u32 = 0x86;
const ID_VIDEO: u32 = 0xE0;
const ID_PIXEL_WIDTH: u32 = 0xB0;
const ID_PIXEL_HEIGHT: u32 = 0xBA;
const ID_ALPHA_MODE: u32 = 0x53C0;

const ID_CLUSTER: u32 = 0x1F43_B675;
const ID_TIMECODE: u32 = 0xE7;
const ID_SIMPLE_BLOCK: u32 = 0xA3;
const ID_BLOCK_GROUP: u32 = 0xA0;
const ID_BLOCK: u32 = 0xA1;
const ID_BLOCK_ADDITIONS: u32 = 0x75A1;
const ID_BLOCK_MORE: u32 = 0xA6;
const ID_BLOCK_ADD_ID: u32 = 0xEE;
const ID_BLOCK_ADDITIONAL: u32 = 0xA5;

/// The video codec carried by the track (selects the Matroska `CodecID`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebmCodec {
    Vp8,
    Vp9,
    Av1,
}

impl WebmCodec {
    fn codec_id(self) -> &'static [u8] {
        match self {
            WebmCodec::Vp8 => b"V_VP8",
            WebmCodec::Vp9 => b"V_VP9",
            WebmCodec::Av1 => b"V_AV01",
        }
    }
}

/// One coded frame to place in the stream.
pub struct WebmFrame<'a> {
    /// The codec bitstream for this frame (a VP9 super-frame, VP8 frame, …).
    pub data: &'a [u8],
    /// Optional alpha-channel VP9 bitstream (`BlockAdditional`, `BlockAddID = 1`).
    pub alpha: Option<&'a [u8]>,
    /// Presentation time in `TimecodeScale` ticks from the start of the segment.
    pub timecode: u64,
    /// Whether this frame is a keyframe (sets the Block / SimpleBlock keyframe flag).
    pub keyframe: bool,
}

/// Parameters for [`mux_webm`].
pub struct WebmParams<'a> {
    pub width: u32,
    pub height: u32,
    pub codec: WebmCodec,
    /// Nanoseconds per `TimecodeScale` tick — 1_000_000 gives millisecond
    /// timecodes (the WebM default).
    pub timecode_scale_ns: u32,
    pub frames: &'a [WebmFrame<'a>],
}

/// Encode `value` as an EBML variable-length integer (used for element sizes and
/// the SimpleBlock track number). The minimal length is chosen; the all-ones
/// pattern (reserved for "unknown size") is never produced.
fn vint(value: u64) -> Vec<u8> {
    let mut len = 1usize;
    while len < 8 && value >= (1u64 << (7 * len)) - 1 {
        len += 1;
    }
    let mut bytes = vec![0u8; len];
    let mut v = value;
    for i in (0..len).rev() {
        bytes[i] = (v & 0xff) as u8;
        v >>= 8;
    }
    bytes[0] |= 1 << (8 - len); // length-marker bit
    bytes
}

/// Minimal big-endian bytes of `id` (its own marker bits give the length).
fn id_bytes(id: u32) -> Vec<u8> {
    let b = id.to_be_bytes();
    let start = b.iter().position(|&x| x != 0).unwrap_or(3);
    b[start..].to_vec()
}

/// Minimal big-endian encoding of an unsigned integer (at least one byte).
fn uint(mut value: u64) -> Vec<u8> {
    if value == 0 {
        return vec![0];
    }
    let mut bytes = Vec::new();
    while value != 0 {
        bytes.push((value & 0xff) as u8);
        value >>= 8;
    }
    bytes.reverse();
    bytes
}

/// One EBML element: `id`, then its size as a VINT, then `payload`.
fn elem(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = id_bytes(id);
    v.extend_from_slice(&vint(payload.len() as u64));
    v.extend_from_slice(payload);
    v
}

fn concat(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

/// Encode `frame_count` solid mid-gray VP9 keyframes and mux them into a
/// `.webm`. Uses the native [`crate::codec::vp9`] encoder — no ffmpeg, no C
/// bindings — so it runs on native and `wasm32`.
///
/// Currently only `64×64` is supported by the gray keyframe path. `fps` must
/// be ≥ 1.
pub fn encode_gray_webm(width: u32, height: u32, frame_count: u32, fps: u32) -> Vec<u8> {
    assert!(frame_count >= 1, "need at least one frame");
    assert!(fps >= 1, "fps must be >= 1");

    let bitstream = crate::codec::vp9::encode_intra_gray(width, height);
    let tick_ms = (1000u64 / fps as u64).max(1);
    let frames: Vec<WebmFrame> = (0..frame_count)
        .map(|i| WebmFrame {
            data: &bitstream,
            alpha: None,
            timecode: i as u64 * tick_ms,
            keyframe: true,
        })
        .collect();

    mux_webm(&WebmParams {
        width,
        height,
        codec: WebmCodec::Vp9,
        timecode_scale_ns: 1_000_000,
        frames: &frames,
    })
}

/// Encode VP9 keyframes from planar YUV (optional alpha) and mux into a `.webm`.
///
/// Each frame is coded independently (all keyframes). Dimensions must be
/// multiples of 8. When any frame carries [`Yuv420Frame::alpha`], the track is
/// marked `AlphaMode = 1` and each cluster uses a `BlockGroup` with the alpha
/// plane coded as a second VP9 bitstream in `BlockAdditional`.
pub fn encode_webm(frames_yuv: &[&crate::codec::hevc::Yuv420Frame], fps: u32) -> Vec<u8> {
    assert!(!frames_yuv.is_empty(), "need at least one frame");
    assert!(fps >= 1, "fps must be >= 1");
    let (w, h) = (frames_yuv[0].width, frames_yuv[0].height);
    for f in frames_yuv {
        assert_eq!((f.width, f.height), (w, h), "all frames must share a size");
    }

    let color: Vec<Vec<u8>> = frames_yuv
        .iter()
        .map(|f| crate::codec::vp9::encode_intra_frame(f).0)
        .collect();

    let alpha: Vec<Option<Vec<u8>>> = frames_yuv
        .iter()
        .map(|f| {
            f.alpha.as_ref().map(|a| {
                assert_eq!(a.len(), (w * h) as usize, "alpha plane size");
                let mut plane = crate::codec::hevc::Yuv420Frame::new(w, h);
                plane.y.copy_from_slice(a);
                plane.u.fill(128);
                plane.v.fill(128);
                crate::codec::vp9::encode_intra_frame(&plane).0
            })
        })
        .collect();

    let tick_ms = (1000u64 / fps as u64).max(1);
    let frames: Vec<WebmFrame> = color
        .iter()
        .zip(alpha.iter())
        .enumerate()
        .map(|(i, (b, a))| WebmFrame {
            data: b,
            alpha: a.as_deref(),
            timecode: i as u64 * tick_ms,
            keyframe: true,
        })
        .collect();

    mux_webm(&WebmParams {
        width: w,
        height: h,
        codec: WebmCodec::Vp9,
        timecode_scale_ns: 1_000_000,
        frames: &frames,
    })
}

/// Mux the frames into a single-track `.webm` byte stream.
pub fn mux_webm(p: &WebmParams) -> Vec<u8> {
    let scale = if p.timecode_scale_ns == 0 {
        1_000_000
    } else {
        p.timecode_scale_ns
    };
    let alpha_mode = p.frames.iter().any(|f| f.alpha.is_some());

    // ---- EBML header (DocType = "webm") ----
    let ebml = elem(
        ID_EBML,
        &concat(&[
            elem(ID_EBML_VERSION, &uint(1)),
            elem(ID_EBML_READ_VERSION, &uint(1)),
            elem(ID_EBML_MAX_ID_LENGTH, &uint(4)),
            elem(ID_EBML_MAX_SIZE_LENGTH, &uint(8)),
            elem(ID_DOC_TYPE, b"webm"),
            elem(ID_DOC_TYPE_VERSION, &uint(2)),
            elem(ID_DOC_TYPE_READ_VERSION, &uint(2)),
        ]),
    );

    // ---- Segment > Info ----
    let last = p.frames.last();
    let duration = last.map(|f| f.timecode + 1).unwrap_or(0) as f64;
    let info = elem(
        ID_INFO,
        &concat(&[
            elem(ID_TIMECODE_SCALE, &uint(scale as u64)),
            elem(ID_DURATION, &duration.to_bits().to_be_bytes()),
            elem(ID_MUXING_APP, b"threers"),
            elem(ID_WRITING_APP, b"threers"),
        ]),
    );

    // ---- Segment > Tracks (one video track) ----
    let mut video_parts = vec![
        elem(ID_PIXEL_WIDTH, &uint(p.width as u64)),
        elem(ID_PIXEL_HEIGHT, &uint(p.height as u64)),
    ];
    if alpha_mode {
        video_parts.push(elem(ID_ALPHA_MODE, &uint(1)));
    }
    let video = elem(ID_VIDEO, &concat(&video_parts));
    let track_entry = elem(
        ID_TRACK_ENTRY,
        &concat(&[
            elem(ID_TRACK_NUMBER, &uint(1)),
            elem(ID_TRACK_UID, &uint(1)),
            elem(ID_TRACK_TYPE, &uint(1)), // 1 = video
            elem(ID_FLAG_LACING, &uint(0)),
            elem(ID_CODEC_ID, p.codec.codec_id()),
            video,
        ]),
    );
    let tracks = elem(ID_TRACKS, &track_entry);

    // ---- Segment > Cluster(s) ----
    // A Block timecode is a signed 16-bit offset from its Cluster's Timecode,
    // so start a fresh Cluster whenever the offset would overflow.
    let mut clusters: Vec<Vec<u8>> = Vec::new();
    let mut i = 0;
    while i < p.frames.len() {
        let base = p.frames[i].timecode;
        let mut blocks = vec![elem(ID_TIMECODE, &uint(base))];
        while i < p.frames.len() {
            let rel = p.frames[i].timecode as i64 - base as i64;
            if rel > i16::MAX as i64 {
                break;
            }
            blocks.push(cluster_frame(&p.frames[i], rel as i16));
            i += 1;
        }
        clusters.push(elem(ID_CLUSTER, &concat(&blocks)));
    }

    let mut segment_children = vec![info, tracks];
    segment_children.extend(clusters);
    let segment = elem(ID_SEGMENT, &concat(&segment_children));

    concat(&[ebml, segment])
}

/// Opaque frame → `SimpleBlock`; alpha frame → `BlockGroup` + `BlockAdditional`.
fn cluster_frame(frame: &WebmFrame, rel_timecode: i16) -> Vec<u8> {
    match frame.alpha {
        None => simple_block(frame, rel_timecode),
        Some(alpha) => block_group_with_alpha(frame, alpha, rel_timecode),
    }
}

/// Block / SimpleBlock payload: track VINT, 16-bit relative timecode, flags, data.
fn block_payload(frame: &WebmFrame, rel_timecode: i16) -> Vec<u8> {
    let mut payload = vint(1); // track number 1
    payload.extend_from_slice(&rel_timecode.to_be_bytes());
    payload.push(if frame.keyframe { 0x80 } else { 0x00 });
    payload.extend_from_slice(frame.data);
    payload
}

/// A SimpleBlock element (no alpha).
fn simple_block(frame: &WebmFrame, rel_timecode: i16) -> Vec<u8> {
    elem(ID_SIMPLE_BLOCK, &block_payload(frame, rel_timecode))
}

/// `BlockGroup` with colour `Block` and alpha in `BlockAdditional` (`BlockAddID = 1`).
fn block_group_with_alpha(frame: &WebmFrame, alpha: &[u8], rel_timecode: i16) -> Vec<u8> {
    let block = elem(ID_BLOCK, &block_payload(frame, rel_timecode));
    let more = elem(
        ID_BLOCK_MORE,
        &concat(&[
            elem(ID_BLOCK_ADD_ID, &uint(1)),
            elem(ID_BLOCK_ADDITIONAL, alpha),
        ]),
    );
    let additions = elem(ID_BLOCK_ADDITIONS, &more);
    elem(ID_BLOCK_GROUP, &concat(&[block, additions]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vint_lengths_and_values() {
        assert_eq!(vint(0), vec![0x80]);
        assert_eq!(vint(1), vec![0x81]);
        assert_eq!(vint(126), vec![0xFE]); // max 1-byte (127 is reserved)
        assert_eq!(vint(127), vec![0x40, 0x7F]); // rolls to 2 bytes
        assert_eq!(vint(0x3FFE), vec![0x7F, 0xFE]); // max 2-byte
    }

    #[test]
    fn id_and_uint_encoding() {
        assert_eq!(id_bytes(ID_SIMPLE_BLOCK), vec![0xA3]);
        assert_eq!(id_bytes(ID_TIMECODE_SCALE), vec![0x2A, 0xD7, 0xB1]);
        assert_eq!(id_bytes(ID_EBML), vec![0x1A, 0x45, 0xDF, 0xA3]);
        assert_eq!(uint(0), vec![0x00]);
        assert_eq!(uint(640), vec![0x02, 0x80]);
        assert_eq!(uint(0x1_0000), vec![0x01, 0x00, 0x00]);
    }

    /// Walk the EBML tree and confirm every element's declared size fits inside
    /// its parent and children tile the payload exactly.
    fn check_tree(buf: &[u8]) {
        fn read_vint(b: &[u8], pos: &mut usize) -> u64 {
            let first = b[*pos];
            let len = first.leading_zeros() as usize + 1;
            let mut v = (first & (0xff >> len)) as u64;
            for k in 1..len {
                v = (v << 8) | b[*pos + k] as u64;
            }
            *pos += len;
            v
        }
        fn walk(b: &[u8], mut pos: usize, end: usize, masters: &[u32]) {
            while pos < end {
                // element ID: length from the marker bit of the first byte
                let id_len = b[pos].leading_zeros() as usize + 1;
                let mut id = 0u32;
                for k in 0..id_len {
                    id = (id << 8) | b[pos + k] as u32;
                }
                pos += id_len;
                let size = read_vint(b, &mut pos) as usize;
                assert!(pos + size <= end, "element {id:X} overflows parent");
                if masters.contains(&id) {
                    walk(b, pos, pos + size, masters);
                }
                pos += size;
            }
            assert_eq!(pos, end, "children tile the parent exactly");
        }
        let masters = [
            ID_EBML,
            ID_SEGMENT,
            ID_INFO,
            ID_TRACKS,
            ID_TRACK_ENTRY,
            ID_VIDEO,
            ID_CLUSTER,
            ID_BLOCK_GROUP,
            ID_BLOCK_ADDITIONS,
            ID_BLOCK_MORE,
        ];
        walk(buf, 0, buf.len(), &masters);
    }

    #[test]
    fn structure_is_well_formed() {
        let f0 = WebmFrame {
            data: &[1, 2, 3, 4],
            alpha: None,
            timecode: 0,
            keyframe: true,
        };
        let f1 = WebmFrame {
            data: &[5, 6, 7],
            alpha: None,
            timecode: 33,
            keyframe: false,
        };
        let out = mux_webm(&WebmParams {
            width: 320,
            height: 240,
            codec: WebmCodec::Vp9,
            timecode_scale_ns: 1_000_000,
            frames: &[f0, f1],
        });
        // Starts with the EBML header ID.
        assert_eq!(&out[..4], &[0x1A, 0x45, 0xDF, 0xA3]);
        check_tree(&out);
    }

    #[test]
    fn frames_split_across_clusters_on_timecode_overflow() {
        // Two frames >32767 ticks apart must land in separate clusters.
        let f0 = WebmFrame {
            data: &[0],
            alpha: None,
            timecode: 0,
            keyframe: true,
        };
        let f1 = WebmFrame {
            data: &[0],
            alpha: None,
            timecode: 100_000,
            keyframe: true,
        };
        let out = mux_webm(&WebmParams {
            width: 16,
            height: 16,
            codec: WebmCodec::Vp9,
            timecode_scale_ns: 1_000_000,
            frames: &[f0, f1],
        });
        check_tree(&out);
        let clusters = out
            .windows(4)
            .filter(|w| *w == ID_CLUSTER.to_be_bytes())
            .count();
        assert_eq!(clusters, 2, "each far-apart frame gets its own cluster");
    }

    #[test]
    fn encode_gray_webm_is_well_formed() {
        let out = encode_gray_webm(64, 64, 2, 30);
        assert_eq!(&out[..4], &[0x1A, 0x45, 0xDF, 0xA3]);
        check_tree(&out);
        // DocType "webm" appears in the EBML header.
        assert!(out.windows(4).any(|w| w == b"webm"));
        assert!(out.windows(5).any(|w| w == b"V_VP9"));
    }

    #[test]
    fn alpha_frame_uses_block_group() {
        let color = [1u8, 2, 3, 4];
        let alpha = [9u8, 8, 7, 6, 5];
        let f = WebmFrame {
            data: &color,
            alpha: Some(&alpha),
            timecode: 0,
            keyframe: true,
        };
        let out = mux_webm(&WebmParams {
            width: 8,
            height: 8,
            codec: WebmCodec::Vp9,
            timecode_scale_ns: 1_000_000,
            frames: &[f],
        });
        check_tree(&out);
        assert!(
            out.windows(2).any(|w| w == [0x53, 0xC0]),
            "AlphaMode present"
        );
        assert!(
            out.windows(2).any(|w| w == [0x75, 0xA1]),
            "BlockAdditions present"
        );
        assert!(out.windows(1).any(|w| w == [0xA0]), "BlockGroup present");
    }
}

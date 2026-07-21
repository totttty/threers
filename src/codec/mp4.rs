//! Minimal ISOBMFF / MP4 muxer for a single HEVC video track (pure Rust, wasm-safe).
//!
//! Enough of the box hierarchy to produce a `.mp4`/`.mov` that AVFoundation and
//! ffmpeg open: `ftyp` + `mdat` + `moov`, with an `hvc1` sample entry carrying an
//! `hvcC` config record (and, for transparent video, Apple's `almo` alpha-mode
//! box). NAL units inside samples are 4-byte length-prefixed (not Annex-B).
//!
//! Layout is `ftyp, mdat, moov` — `mdat` before `moov` so chunk offsets (`stco`)
//! are fixed before `moov`'s size is known.

/// Wrap `payload` in a box with `type_` (a 4-byte tag).
fn bx(type_: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(8 + payload.len());
    v.extend_from_slice(&((8 + payload.len()) as u32).to_be_bytes());
    v.extend_from_slice(type_);
    v.extend_from_slice(payload);
    v
}

/// A full-box (`version` + 24-bit `flags`) wrapping `payload`.
fn fullbox(type_: &[u8; 4], version: u8, flags: u32, payload: &[u8]) -> Vec<u8> {
    let mut p = Vec::with_capacity(4 + payload.len());
    p.push(version);
    p.extend_from_slice(&flags.to_be_bytes()[1..]);
    p.extend_from_slice(payload);
    bx(type_, &p)
}

fn concat(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

/// Parameters for [`mux_hevc`].
pub struct Mp4Params<'a> {
    pub width: u32,
    pub height: u32,
    /// Media timescale (ticks per second). Apple uses 600.
    pub timescale: u32,
    /// Duration of each sample in `timescale` ticks.
    pub frame_duration: u32,
    /// The `hvcC` box payload (without its size/type header) — see
    /// [`crate::codec::hevc::hvcc`].
    pub hvcc_payload: &'a [u8],
    /// Optional Apple `almo` alpha-mode box payload (4 bytes) to append to the
    /// sample entry, signaling a transparent track.
    pub almo_payload: Option<&'a [u8]>,
    /// Per-frame sample data: each entry is that frame's NAL units, already
    /// 4-byte length-prefixed and concatenated.
    pub samples: &'a [Vec<u8>],
}

/// Mux one HEVC video track into a `.mp4`/`.mov` byte stream.
pub fn mux_hevc(p: &Mp4Params) -> Vec<u8> {
    let num_samples = p.samples.len().max(1) as u32;
    let total_duration = p.frame_duration * num_samples;

    // ---- ftyp ----
    let ftyp = bx(b"ftyp", &{
        let mut v = Vec::new();
        v.extend_from_slice(b"isom"); // major_brand
        v.extend_from_slice(&0u32.to_be_bytes()); // minor_version
        for brand in [b"isom", b"iso2", b"mp41", b"hvc1"] {
            v.extend_from_slice(brand);
        }
        v
    });

    // ---- mdat (concatenated samples) ----
    let mdat_payload = concat(p.samples);
    let mdat = bx(b"mdat", &mdat_payload);
    let mdat_data_offset = (ftyp.len() + 8) as u32; // sample data starts after mdat header

    // ---- sample tables ----
    let stsd = {
        let hvcc = bx(b"hvcC", p.hvcc_payload);
        let mut sample_entry = Vec::new();
        sample_entry.extend_from_slice(&[0u8; 6]); // reserved
        sample_entry.extend_from_slice(&1u16.to_be_bytes()); // data_reference_index
        sample_entry.extend_from_slice(&[0u8; 16]); // pre_defined + reserved (2+2+12)
        sample_entry.extend_from_slice(&(p.width as u16).to_be_bytes());
        sample_entry.extend_from_slice(&(p.height as u16).to_be_bytes());
        sample_entry.extend_from_slice(&0x0048_0000u32.to_be_bytes()); // horizresolution 72dpi
        sample_entry.extend_from_slice(&0x0048_0000u32.to_be_bytes()); // vertresolution
        sample_entry.extend_from_slice(&0u32.to_be_bytes()); // reserved
        sample_entry.extend_from_slice(&1u16.to_be_bytes()); // frame_count
        sample_entry.extend_from_slice(&[0u8; 32]); // compressorname
        sample_entry.extend_from_slice(&0x0018u16.to_be_bytes()); // depth
        sample_entry.extend_from_slice(&0xFFFFu16.to_be_bytes()); // pre_defined -1
        sample_entry.extend_from_slice(&hvcc);
        if let Some(almo) = p.almo_payload {
            sample_entry.extend_from_slice(&bx(b"almo", almo));
        }
        let hvc1 = bx(b"hvc1", &sample_entry);
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u32.to_be_bytes()); // entry_count
        payload.extend_from_slice(&hvc1);
        fullbox(b"stsd", 0, 0, &payload)
    };

    let stts = fullbox(b"stts", 0, 0, &{
        let mut v = Vec::new();
        v.extend_from_slice(&1u32.to_be_bytes()); // entry_count
        v.extend_from_slice(&num_samples.to_be_bytes()); // sample_count
        v.extend_from_slice(&p.frame_duration.to_be_bytes()); // sample_delta
        v
    });

    // All samples are IDR → all sync samples: stss lists them all.
    let stss = fullbox(b"stss", 0, 0, &{
        let mut v = Vec::new();
        v.extend_from_slice(&num_samples.to_be_bytes());
        for i in 1..=num_samples {
            v.extend_from_slice(&i.to_be_bytes());
        }
        v
    });

    let stsc = fullbox(b"stsc", 0, 0, &{
        let mut v = Vec::new();
        v.extend_from_slice(&1u32.to_be_bytes()); // entry_count
        v.extend_from_slice(&1u32.to_be_bytes()); // first_chunk
        v.extend_from_slice(&num_samples.to_be_bytes()); // samples_per_chunk
        v.extend_from_slice(&1u32.to_be_bytes()); // sample_description_index
        v
    });

    let stsz = fullbox(b"stsz", 0, 0, &{
        let mut v = Vec::new();
        v.extend_from_slice(&0u32.to_be_bytes()); // sample_size 0 = per-sample
        v.extend_from_slice(&num_samples.to_be_bytes()); // sample_count
        for s in p.samples {
            v.extend_from_slice(&(s.len() as u32).to_be_bytes());
        }
        v
    });

    // Single chunk holding all samples, located at the start of mdat's payload.
    let stco = fullbox(b"stco", 0, 0, &{
        let mut v = Vec::new();
        v.extend_from_slice(&1u32.to_be_bytes()); // entry_count
        v.extend_from_slice(&mdat_data_offset.to_be_bytes());
        v
    });

    let stbl = bx(b"stbl", &concat(&[stsd, stts, stss, stsc, stsz, stco]));

    // ---- minf ----
    let vmhd = fullbox(b"vmhd", 0, 1, &[0u8; 8]); // graphicsmode + opcolor
    let dref = fullbox(b"dref", 0, 0, &{
        let mut v = Vec::new();
        v.extend_from_slice(&1u32.to_be_bytes()); // entry_count
        v.extend_from_slice(&fullbox(b"url ", 0, 1, &[])); // self-contained
        v
    });
    let dinf = bx(b"dinf", &dref);
    let vid_hdlr = fullbox(b"hdlr", 0, 0, &{
        let mut v = Vec::new();
        v.extend_from_slice(&0u32.to_be_bytes()); // pre_defined
        v.extend_from_slice(b"vide"); // handler_type
        v.extend_from_slice(&[0u8; 12]); // reserved
        v.extend_from_slice(b"threers\0"); // name
        v
    });
    let minf = bx(b"minf", &concat(&[vmhd, vid_hdlr, dinf, stbl]));

    // ---- mdia ----
    let mdhd = fullbox(b"mdhd", 0, 0, &{
        let mut v = Vec::new();
        v.extend_from_slice(&0u32.to_be_bytes()); // creation_time
        v.extend_from_slice(&0u32.to_be_bytes()); // modification_time
        v.extend_from_slice(&p.timescale.to_be_bytes());
        v.extend_from_slice(&total_duration.to_be_bytes());
        v.extend_from_slice(&0x55c4u16.to_be_bytes()); // language 'und'
        v.extend_from_slice(&0u16.to_be_bytes()); // pre_defined
        v
    });
    let mdia_hdlr = fullbox(b"hdlr", 0, 0, &{
        let mut v = Vec::new();
        v.extend_from_slice(&0u32.to_be_bytes());
        v.extend_from_slice(b"vide");
        v.extend_from_slice(&[0u8; 12]);
        v.extend_from_slice(b"threers\0");
        v
    });
    let mdia = bx(b"mdia", &concat(&[mdhd, mdia_hdlr, minf]));

    // ---- trak ----
    let tkhd = fullbox(b"tkhd", 0, 7 /* enabled|in_movie|in_preview */, &{
        let mut v = Vec::new();
        v.extend_from_slice(&0u32.to_be_bytes()); // creation_time
        v.extend_from_slice(&0u32.to_be_bytes()); // modification_time
        v.extend_from_slice(&1u32.to_be_bytes()); // track_ID
        v.extend_from_slice(&0u32.to_be_bytes()); // reserved
        v.extend_from_slice(&total_duration.to_be_bytes());
        v.extend_from_slice(&[0u8; 8]); // reserved
        v.extend_from_slice(&0u16.to_be_bytes()); // layer
        v.extend_from_slice(&0u16.to_be_bytes()); // alternate_group
        v.extend_from_slice(&0u16.to_be_bytes()); // volume
        v.extend_from_slice(&0u16.to_be_bytes()); // reserved
        v.extend_from_slice(&UNITY_MATRIX);
        v.extend_from_slice(&((p.width << 16) as u32).to_be_bytes()); // width 16.16
        v.extend_from_slice(&((p.height << 16) as u32).to_be_bytes()); // height 16.16
        v
    });
    let trak = bx(b"trak", &concat(&[tkhd, mdia]));

    // ---- mvhd ----
    let mvhd = fullbox(b"mvhd", 0, 0, &{
        let mut v = Vec::new();
        v.extend_from_slice(&0u32.to_be_bytes()); // creation_time
        v.extend_from_slice(&0u32.to_be_bytes()); // modification_time
        v.extend_from_slice(&p.timescale.to_be_bytes());
        v.extend_from_slice(&total_duration.to_be_bytes());
        v.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // rate 1.0
        v.extend_from_slice(&0x0100u16.to_be_bytes()); // volume 1.0
        v.extend_from_slice(&0u16.to_be_bytes()); // reserved
        v.extend_from_slice(&[0u8; 8]); // reserved
        v.extend_from_slice(&UNITY_MATRIX);
        v.extend_from_slice(&[0u8; 24]); // pre_defined
        v.extend_from_slice(&2u32.to_be_bytes()); // next_track_ID
        v
    });
    let moov = bx(b"moov", &concat(&[mvhd, trak]));

    concat(&[ftyp, mdat, moov])
}

/// The identity transformation matrix used in `tkhd`/`mvhd` (16.16 / 2.30 fixed).
const UNITY_MATRIX: [u8; 36] = [
    0x00, 0x01, 0x00, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, //
    0, 0, 0, 0, 0x00, 0x01, 0x00, 0x00, 0, 0, 0, 0, //
    0, 0, 0, 0, 0, 0, 0, 0, 0x40, 0x00, 0x00, 0x00,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boxes_are_well_formed() {
        let sample = vec![0u8; 40];
        let out = mux_hevc(&Mp4Params {
            width: 64,
            height: 64,
            timescale: 600,
            frame_duration: 20,
            hvcc_payload: &[0u8; 30],
            almo_payload: Some(&[0, 0, 1, 2]),
            samples: &[sample],
        });
        assert_eq!(&out[4..8], b"ftyp");
        // Top-level boxes must tile exactly.
        let mut o = 0;
        let mut tags = Vec::new();
        while o + 8 <= out.len() {
            let size = u32::from_be_bytes([out[o], out[o + 1], out[o + 2], out[o + 3]]) as usize;
            tags.push(String::from_utf8_lossy(&out[o + 4..o + 8]).to_string());
            assert!(size >= 8 && o + size <= out.len(), "box {size} at {o}");
            o += size;
        }
        assert_eq!(o, out.len(), "top-level boxes tile the file");
        assert_eq!(tags, vec!["ftyp", "mdat", "moov"]);
    }
}

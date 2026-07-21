//! `HEVCDecoderConfigurationRecord` (`hvcC`) builder — ISO/IEC 14496-15 §8.3.3.1.
//!
//! Packs the parameter-set NAL units (VPS/SPS/PPS and, for transparent video, the
//! `alpha_channel_info` SEI) into the config record an MP4 `hvc1` sample entry
//! carries. NAL units here keep their 2-byte header and emulation prevention and
//! are length-prefixed with 2 bytes.

/// One `hvcC` NAL array: a NAL-unit type plus the NAL units of that type.
pub struct HvccArray<'a> {
    /// `NAL_unit_type` (e.g. 32 VPS, 33 SPS, 34 PPS, 39 prefix SEI).
    pub nal_type: u8,
    /// Whether all NALs of this type are in the record (`array_completeness`).
    pub complete: bool,
    /// Full NAL units (2-byte header + EBSP).
    pub nals: &'a [Vec<u8>],
}

/// General profile/format fields for the record (mirrors `profile_tier_level`).
pub struct HvccProfile {
    pub profile_idc: u8,
    pub level_idc: u8,
    pub chroma_format_idc: u8,
    pub bit_depth_luma_minus8: u8,
    pub bit_depth_chroma_minus8: u8,
    /// 6-byte `general_constraint_indicator_flags`.
    pub constraint_flags: [u8; 6],
    /// 4-byte `general_profile_compatibility_flags`.
    pub compat_flags: [u8; 4],
}

impl HvccProfile {
    /// Main profile, 8-bit 4:2:0 (matches [`crate::codec::hevc::params`] defaults).
    pub fn main_420(level_idc: u8) -> Self {
        // compat bit for profile 1; progressive+frame_only in the constraint bytes.
        Self {
            profile_idc: 1,
            level_idc,
            chroma_format_idc: 1,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            constraint_flags: [0x90, 0x00, 0x00, 0x00, 0x00, 0x00],
            compat_flags: [0x60, 0x00, 0x00, 0x00],
        }
    }
}

/// Build the `hvcC` box payload (without the 8-byte box header).
pub fn build_hvcc(profile: &HvccProfile, arrays: &[HvccArray]) -> Vec<u8> {
    let mut v = Vec::new();
    v.push(1); // configurationVersion
    v.push(profile.profile_idc & 0x1F); // profile_space(0)|tier(0)|profile_idc
    v.extend_from_slice(&profile.compat_flags);
    v.extend_from_slice(&profile.constraint_flags);
    v.push(profile.level_idc);
    v.extend_from_slice(&[0xF0, 0x00]); // reserved|min_spatial_segmentation_idc = 0
    v.push(0xFC); // reserved|parallelismType = 0
    v.push(0xFC | (profile.chroma_format_idc & 3)); // reserved|chromaFormat
    v.push(0xF8 | (profile.bit_depth_luma_minus8 & 7)); // reserved|bitDepthLumaMinus8
    v.push(0xF8 | (profile.bit_depth_chroma_minus8 & 7)); // reserved|bitDepthChromaMinus8
    v.extend_from_slice(&[0x00, 0x00]); // avgFrameRate
                                        // constantFrameRate(2)=0 | numTemporalLayers(3)=1 | temporalIdNested(1)=0 |
                                        // lengthSizeMinusOne(2)=3  →  0b0000_1011 = 0x0B (4-byte NAL length prefixes).
    v.push(0x0B);
    v.push(arrays.len() as u8); // numOfArrays
    for a in arrays {
        v.push(((a.complete as u8) << 7) | (a.nal_type & 0x3F)); // array_completeness|reserved|type
        v.extend_from_slice(&(a.nals.len() as u16).to_be_bytes()); // numNalus
        for nal in a.nals {
            v.extend_from_slice(&(nal.len() as u16).to_be_bytes());
            v.extend_from_slice(nal);
        }
    }
    v
}

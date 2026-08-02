//! Stable serialization for baked light-probe grids.
//!
//! The cache stores renderer-independent L2 spherical-harmonic coefficients.
//! GPU textures are deliberately rebuilt on load so one cache works across
//! native and WebGPU adapters with different row-alignment limits.

use crate::math::Vector3;
use std::fmt;

pub const MAX_LIGHT_PROBE_GRID_PROBES: usize = 2048;
pub const LIGHT_PROBE_COEFFICIENTS_PER_PROBE: usize = 9 * 4;

const MAGIC: [u8; 8] = *b"THRLPB\0\0";
const FORMAT_VERSION: u16 = 1;
const HEADER_LEN: usize = 88;
const CHECKSUM_OFFSET: usize = 76;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightProbeBakeSettings {
    pub cubemap_size: u32,
    pub near: f32,
    pub far: f32,
    pub bounces: u32,
}

impl LightProbeBakeSettings {
    fn validate(self) -> Result<(), LightProbeCacheError> {
        if self.cubemap_size < 2 {
            return Err(LightProbeCacheError::InvalidSettings(
                "cubemap size must be at least 2",
            ));
        }
        if !self.near.is_finite()
            || !self.far.is_finite()
            || self.near <= 0.0
            || self.far <= self.near
        {
            return Err(LightProbeCacheError::InvalidSettings(
                "near/far must be finite, positive, and ordered",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BakedLightProbeGrid {
    pub resolution: [u32; 3],
    pub min: Vector3,
    pub max: Vector3,
    pub settings: LightProbeBakeSettings,
    pub coefficients: Vec<f32>,
    fingerprint: u64,
}

impl BakedLightProbeGrid {
    pub fn new(
        resolution: [u32; 3],
        min: Vector3,
        max: Vector3,
        settings: LightProbeBakeSettings,
        coefficients: Vec<f32>,
        cache_key: &str,
    ) -> Result<Self, LightProbeCacheError> {
        if cache_key.is_empty() {
            return Err(LightProbeCacheError::EmptyCacheKey);
        }
        validate_grid(resolution, min, max, settings, &coefficients)?;
        Ok(Self {
            resolution,
            min,
            max,
            settings,
            coefficients,
            fingerprint: cache_fingerprint(cache_key),
        })
    }

    pub fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, LightProbeCacheError> {
        validate_grid(
            self.resolution,
            self.min,
            self.max,
            self.settings,
            &self.coefficients,
        )?;
        let payload_len = self
            .coefficients
            .len()
            .checked_mul(std::mem::size_of::<f32>())
            .ok_or(LightProbeCacheError::LengthOverflow)?;
        let payload_len_u32 =
            u32::try_from(payload_len).map_err(|_| LightProbeCacheError::LengthOverflow)?;
        let coefficient_count = u32::try_from(self.coefficients.len())
            .map_err(|_| LightProbeCacheError::LengthOverflow)?;
        let mut bytes = vec![0u8; HEADER_LEN + payload_len];
        bytes[0..8].copy_from_slice(&MAGIC);
        put_u16(&mut bytes, 8, FORMAT_VERSION);
        put_u16(&mut bytes, 10, HEADER_LEN as u16);
        put_u32(&mut bytes, 12, payload_len_u32);
        put_u32(&mut bytes, 16, self.resolution[0]);
        put_u32(&mut bytes, 20, self.resolution[1]);
        put_u32(&mut bytes, 24, self.resolution[2]);
        for (index, value) in [
            self.min.x, self.min.y, self.min.z, self.max.x, self.max.y, self.max.z,
        ]
        .into_iter()
        .enumerate()
        {
            put_f32(&mut bytes, 28 + index * 4, value);
        }
        put_u32(&mut bytes, 52, self.settings.cubemap_size);
        put_u32(&mut bytes, 56, self.settings.bounces);
        put_f32(&mut bytes, 60, self.settings.near);
        put_f32(&mut bytes, 64, self.settings.far);
        put_u64(&mut bytes, 68, self.fingerprint);
        put_u32(&mut bytes, 80, coefficient_count);
        for (index, value) in self.coefficients.iter().enumerate() {
            put_f32(&mut bytes, HEADER_LEN + index * 4, *value);
        }
        let checksum = cache_crc32(&bytes);
        put_u32(&mut bytes, CHECKSUM_OFFSET, checksum);
        Ok(bytes)
    }

    pub fn from_bytes(
        bytes: &[u8],
        expected_cache_key: &str,
    ) -> Result<Self, LightProbeCacheError> {
        if expected_cache_key.is_empty() {
            return Err(LightProbeCacheError::EmptyCacheKey);
        }
        if bytes.len() < HEADER_LEN {
            return Err(LightProbeCacheError::Truncated);
        }
        if bytes[0..8] != MAGIC {
            return Err(LightProbeCacheError::InvalidMagic);
        }
        let version = get_u16(bytes, 8);
        if version != FORMAT_VERSION {
            return Err(LightProbeCacheError::UnsupportedVersion(version));
        }
        let header_len = get_u16(bytes, 10) as usize;
        if header_len != HEADER_LEN {
            return Err(LightProbeCacheError::InvalidHeaderLength(header_len));
        }
        let payload_len = get_u32(bytes, 12) as usize;
        let expected_len = HEADER_LEN
            .checked_add(payload_len)
            .ok_or(LightProbeCacheError::LengthOverflow)?;
        if bytes.len() != expected_len {
            return Err(LightProbeCacheError::LengthMismatch {
                expected: expected_len,
                actual: bytes.len(),
            });
        }
        let resolution = [get_u32(bytes, 16), get_u32(bytes, 20), get_u32(bytes, 24)];
        let probe_count = checked_probe_count(resolution)?;
        let coefficient_count = get_u32(bytes, 80) as usize;
        let expected_coefficient_count = probe_count
            .checked_mul(LIGHT_PROBE_COEFFICIENTS_PER_PROBE)
            .ok_or(LightProbeCacheError::LengthOverflow)?;
        if coefficient_count != expected_coefficient_count {
            return Err(LightProbeCacheError::InvalidCoefficientCount);
        }
        let expected_payload_len = coefficient_count
            .checked_mul(std::mem::size_of::<f32>())
            .ok_or(LightProbeCacheError::LengthOverflow)?;
        if expected_payload_len != payload_len {
            return Err(LightProbeCacheError::InvalidCoefficientCount);
        }
        let stored_checksum = get_u32(bytes, CHECKSUM_OFFSET);
        if cache_crc32(bytes) != stored_checksum {
            return Err(LightProbeCacheError::ChecksumMismatch);
        }
        let fingerprint = get_u64(bytes, 68);
        let expected_fingerprint = cache_fingerprint(expected_cache_key);
        if fingerprint != expected_fingerprint {
            return Err(LightProbeCacheError::FingerprintMismatch);
        }

        let min = Vector3::new(get_f32(bytes, 28), get_f32(bytes, 32), get_f32(bytes, 36));
        let max = Vector3::new(get_f32(bytes, 40), get_f32(bytes, 44), get_f32(bytes, 48));
        let settings = LightProbeBakeSettings {
            cubemap_size: get_u32(bytes, 52),
            bounces: get_u32(bytes, 56),
            near: get_f32(bytes, 60),
            far: get_f32(bytes, 64),
        };
        let mut coefficients = Vec::with_capacity(coefficient_count);
        for offset in (HEADER_LEN..bytes.len()).step_by(4) {
            coefficients.push(get_f32(bytes, offset));
        }
        validate_grid(resolution, min, max, settings, &coefficients)?;
        Ok(Self {
            resolution,
            min,
            max,
            settings,
            coefficients,
            fingerprint,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LightProbeCacheError {
    EmptyCacheKey,
    InvalidMagic,
    UnsupportedVersion(u16),
    InvalidHeaderLength(usize),
    Truncated,
    LengthOverflow,
    LengthMismatch { expected: usize, actual: usize },
    ChecksumMismatch,
    FingerprintMismatch,
    InvalidResolution,
    TooManyProbes(usize),
    InvalidBounds,
    InvalidSettings(&'static str),
    InvalidCoefficientCount,
    NonFiniteCoefficient,
}

impl fmt::Display for LightProbeCacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCacheKey => write!(f, "light-probe cache key must not be empty"),
            Self::InvalidMagic => write!(f, "invalid light-probe cache magic"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported light-probe cache version {version}")
            }
            Self::InvalidHeaderLength(length) => {
                write!(f, "invalid light-probe cache header length {length}")
            }
            Self::Truncated => write!(f, "truncated light-probe cache"),
            Self::LengthOverflow => write!(f, "light-probe cache length overflow"),
            Self::LengthMismatch { expected, actual } => {
                write!(
                    f,
                    "light-probe cache length is {actual}; expected {expected}"
                )
            }
            Self::ChecksumMismatch => write!(f, "light-probe cache checksum mismatch"),
            Self::FingerprintMismatch => write!(f, "light-probe cache fingerprint mismatch"),
            Self::InvalidResolution => {
                write!(f, "light-probe resolution must be at least 2 on every axis")
            }
            Self::TooManyProbes(count) => write!(
                f,
                "light-probe grid has {count} probes; maximum is {MAX_LIGHT_PROBE_GRID_PROBES}"
            ),
            Self::InvalidBounds => {
                write!(
                    f,
                    "light-probe bounds must be finite and have positive extent"
                )
            }
            Self::InvalidSettings(message) => {
                write!(f, "invalid light-probe bake settings: {message}")
            }
            Self::InvalidCoefficientCount => write!(f, "invalid light-probe coefficient count"),
            Self::NonFiniteCoefficient => {
                write!(f, "light-probe coefficients must all be finite")
            }
        }
    }
}

impl std::error::Error for LightProbeCacheError {}

fn validate_grid(
    resolution: [u32; 3],
    min: Vector3,
    max: Vector3,
    settings: LightProbeBakeSettings,
    coefficients: &[f32],
) -> Result<(), LightProbeCacheError> {
    let probe_count = checked_probe_count(resolution)?;
    let expected = probe_count
        .checked_mul(LIGHT_PROBE_COEFFICIENTS_PER_PROBE)
        .ok_or(LightProbeCacheError::LengthOverflow)?;
    if coefficients.len() != expected {
        return Err(LightProbeCacheError::InvalidCoefficientCount);
    }
    if coefficients.iter().any(|value| !value.is_finite()) {
        return Err(LightProbeCacheError::NonFiniteCoefficient);
    }
    let bounds = [min.x, min.y, min.z, max.x, max.y, max.z];
    if bounds.iter().any(|value| !value.is_finite())
        || max.x <= min.x
        || max.y <= min.y
        || max.z <= min.z
    {
        return Err(LightProbeCacheError::InvalidBounds);
    }
    settings.validate()
}

fn checked_probe_count(resolution: [u32; 3]) -> Result<usize, LightProbeCacheError> {
    if resolution.iter().any(|&value| value < 2) {
        return Err(LightProbeCacheError::InvalidResolution);
    }
    let probe_count = resolution
        .iter()
        .try_fold(1usize, |count, &value| count.checked_mul(value as usize))
        .ok_or(LightProbeCacheError::LengthOverflow)?;
    if probe_count > MAX_LIGHT_PROBE_GRID_PROBES {
        return Err(LightProbeCacheError::TooManyProbes(probe_count));
    }
    Ok(probe_count)
}

fn cache_fingerprint(cache_key: &str) -> u64 {
    // FNV-1a is used only as a stable cache identity, not for security.
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in cache_key.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn cache_crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for (index, &stored_byte) in bytes.iter().enumerate() {
        let byte = if (CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4).contains(&index) {
            0
        } else {
            stored_byte
        };
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320u32 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_f32(bytes: &mut [u8], offset: usize, value: f32) {
    put_u32(bytes, offset, value.to_bits());
}

fn get_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        bytes[offset..offset + 2]
            .try_into()
            .expect("validated header"),
    )
}

fn get_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("validated length"),
    )
}

fn get_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("validated header"),
    )
}

fn get_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_bits(get_u32(bytes, offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> BakedLightProbeGrid {
        let resolution = [2, 2, 2];
        let coefficients = (0..2 * 2 * 2 * LIGHT_PROBE_COEFFICIENTS_PER_PROBE)
            .map(|index| index as f32 * 0.125 - 4.0)
            .collect();
        BakedLightProbeGrid::new(
            resolution,
            Vector3::new(-1.0, -2.0, -3.0),
            Vector3::new(1.0, 2.0, 3.0),
            LightProbeBakeSettings {
                cubemap_size: 32,
                near: 0.05,
                far: 20.0,
                bounces: 1,
            },
            coefficients,
            "fixture-v1",
        )
        .unwrap()
    }

    #[test]
    fn cache_round_trip_preserves_float_bits_and_metadata() {
        let original = fixture();
        let bytes = original.to_bytes().unwrap();
        let decoded = BakedLightProbeGrid::from_bytes(&bytes, "fixture-v1").unwrap();
        assert_eq!(decoded, original);
        assert!(decoded
            .coefficients
            .iter()
            .zip(&original.coefficients)
            .all(|(a, b)| a.to_bits() == b.to_bits()));
    }

    #[test]
    fn cache_rejects_stale_scene_fingerprint() {
        let bytes = fixture().to_bytes().unwrap();
        assert_eq!(
            BakedLightProbeGrid::from_bytes(&bytes, "fixture-v2").unwrap_err(),
            LightProbeCacheError::FingerprintMismatch
        );
    }

    #[test]
    fn cache_rejects_corrupted_payload() {
        let mut bytes = fixture().to_bytes().unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0x80;
        assert_eq!(
            BakedLightProbeGrid::from_bytes(&bytes, "fixture-v1").unwrap_err(),
            LightProbeCacheError::ChecksumMismatch
        );
    }

    #[test]
    fn cache_rejects_oversized_resolution_before_payload_allocation() {
        let mut bytes = fixture().to_bytes().unwrap();
        put_u32(&mut bytes, 16, 1000);
        put_u32(&mut bytes, CHECKSUM_OFFSET, 0);
        let checksum = cache_crc32(&bytes);
        put_u32(&mut bytes, CHECKSUM_OFFSET, checksum);
        assert_eq!(
            BakedLightProbeGrid::from_bytes(&bytes, "fixture-v1").unwrap_err(),
            LightProbeCacheError::TooManyProbes(4_000)
        );
    }

    #[test]
    fn cache_rejects_non_finite_coefficients() {
        let mut coefficients = vec![0.0; 8 * LIGHT_PROBE_COEFFICIENTS_PER_PROBE];
        coefficients[5] = f32::NAN;
        assert_eq!(
            BakedLightProbeGrid::new(
                [2, 2, 2],
                Vector3::new(-1.0, -1.0, -1.0),
                Vector3::new(1.0, 1.0, 1.0),
                LightProbeBakeSettings {
                    cubemap_size: 8,
                    near: 0.1,
                    far: 10.0,
                    bounces: 0,
                },
                coefficients,
                "invalid",
            )
            .unwrap_err(),
            LightProbeCacheError::NonFiniteCoefficient
        );
    }

}

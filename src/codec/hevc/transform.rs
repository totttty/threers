//! HEVC core integer transforms (H.265 §8.6.4): the DCT-II approximations used
//! for residual coding, plus the alternate DST-VII used for 4×4 intra luma.
//!
//! The **inverse** transform must be bit-exact to the spec (the decoder runs it,
//! and the encoder mirrors it to reconstruct neighbors for intra prediction).
//! The **forward** transform is the encoder's own analysis step; here it is the
//! matching orthogonal partner so `forward → inverse` round-trips.
//!
//! Sizes 4 and 8 are implemented (enough for a first compressed encoder with
//! transform blocks ≤ 8×8); 16/32 follow the same nested-matrix pattern.
//!
//! Pure integer math over slices — identical on native and `wasm32`.

/// 4×4 DCT-II basis (`transMatrix`, H.265 Table in §8.6.4.2).
pub const DCT4: [[i32; 4]; 4] = [
    [64, 64, 64, 64],
    [83, 36, -36, -83],
    [64, -64, -64, 64],
    [36, -83, 83, -36],
];

/// 4×4 DST-VII basis (used for 4×4 intra luma residual).
pub const DST4: [[i32; 4]; 4] = [
    [29, 55, 74, 84],
    [74, 74, 0, -74],
    [84, -29, -74, 55],
    [55, -84, 74, -29],
];

/// 16×16 DCT-II basis. Rows `2i` (cols 0..8) reproduce [`DCT8`] (nested design).
pub const DCT16: [[i32; 16]; 16] = [
    [
        64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64,
    ],
    [
        90, 87, 80, 70, 57, 43, 25, 9, -9, -25, -43, -57, -70, -80, -87, -90,
    ],
    [
        89, 75, 50, 18, -18, -50, -75, -89, -89, -75, -50, -18, 18, 50, 75, 89,
    ],
    [
        87, 57, 9, -43, -80, -90, -70, -25, 25, 70, 90, 80, 43, -9, -57, -87,
    ],
    [
        83, 36, -36, -83, -83, -36, 36, 83, 83, 36, -36, -83, -83, -36, 36, 83,
    ],
    [
        80, 9, -70, -87, -25, 57, 90, 43, -43, -90, -57, 25, 87, 70, -9, -80,
    ],
    [
        75, -18, -89, -50, 50, 89, 18, -75, -75, 18, 89, 50, -50, -89, -18, 75,
    ],
    [
        70, -43, -87, 9, 90, 25, -80, -57, 57, 80, -25, -90, -9, 87, 43, -70,
    ],
    [
        64, -64, -64, 64, 64, -64, -64, 64, 64, -64, -64, 64, 64, -64, -64, 64,
    ],
    [
        57, -80, -25, 90, -9, -87, 43, 70, -70, -43, 87, 9, -90, 25, 80, -57,
    ],
    [
        50, -89, 18, 75, -75, -18, 89, -50, -50, 89, -18, -75, 75, 18, -89, 50,
    ],
    [
        43, -90, 57, 25, -87, 70, 9, -80, 80, -9, -70, 87, -25, -57, 90, -43,
    ],
    [
        36, -83, 83, -36, -36, 83, -83, 36, 36, -83, 83, -36, -36, 83, -83, 36,
    ],
    [
        25, -70, 90, -80, 43, 9, -57, 87, -87, 57, -9, -43, 80, -90, 70, -25,
    ],
    [
        18, -50, 75, -89, 89, -75, 50, -18, -18, 50, -75, 89, -89, 75, -50, 18,
    ],
    [
        9, -25, 43, -57, 70, -80, 87, -90, 90, -87, 80, -70, 57, -43, 25, -9,
    ],
];

/// 8×8 DCT-II basis.
pub const DCT8: [[i32; 8]; 8] = [
    [64, 64, 64, 64, 64, 64, 64, 64],
    [89, 75, 50, 18, -18, -50, -75, -89],
    [83, 36, -36, -83, -83, -36, 36, 83],
    [75, -18, -89, -50, 50, 89, 18, -75],
    [64, -64, -64, 64, 64, -64, -64, 64],
    [50, -89, 18, 75, -75, -18, 89, -50],
    [36, -83, 83, -36, -36, 83, -83, 36],
    [18, -50, 75, -89, 89, -75, 50, -18],
];

/// Which transform to apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformKind {
    /// DCT-II — all sizes.
    Dct,
    /// DST-VII — 4×4 only (intra luma).
    Dst,
}

fn matrix(size: usize, kind: TransformKind) -> &'static [&'static [i32]] {
    // Return rows as slices via small static wrappers.
    match (size, kind) {
        (4, TransformKind::Dst) => DST4_ROWS,
        (4, TransformKind::Dct) => DCT4_ROWS,
        (8, TransformKind::Dct) => DCT8_ROWS,
        (16, TransformKind::Dct) => DCT16_ROWS,
        _ => panic!("unsupported transform size {size}/{kind:?}"),
    }
}

const DCT16_ROWS: &[&[i32]] = &[
    &DCT16[0], &DCT16[1], &DCT16[2], &DCT16[3], &DCT16[4], &DCT16[5], &DCT16[6], &DCT16[7],
    &DCT16[8], &DCT16[9], &DCT16[10], &DCT16[11], &DCT16[12], &DCT16[13], &DCT16[14], &DCT16[15],
];

const DCT4_ROWS: &[&[i32]] = &[&DCT4[0], &DCT4[1], &DCT4[2], &DCT4[3]];
const DST4_ROWS: &[&[i32]] = &[&DST4[0], &DST4[1], &DST4[2], &DST4[3]];
const DCT8_ROWS: &[&[i32]] = &[
    &DCT8[0], &DCT8[1], &DCT8[2], &DCT8[3], &DCT8[4], &DCT8[5], &DCT8[6], &DCT8[7],
];

#[inline]
fn log2(n: usize) -> u32 {
    n.trailing_zeros()
}

#[inline]
fn clip16(v: i64) -> i64 {
    v.clamp(-32768, 32767)
}

/// Forward 2D transform of an `n×n` residual block (row-major), producing
/// coefficients (row-major). `n` ∈ {4, 8}. Shifts assume 8-bit samples.
pub fn forward(residual: &[i32], n: usize, kind: TransformKind) -> Vec<i32> {
    let m = matrix(n, kind);
    let shift1 = log2(n) + 8 - 9; // = log2(n) - 1
    let shift2 = log2(n) + 6;
    let add1 = 1i64 << (shift1.max(1) - 1);
    let add2 = 1i64 << (shift2 - 1);

    // Stage 1: transform rows (horizontal).
    let mut tmp = vec![0i64; n * n];
    for r in 0..n {
        for i in 0..n {
            let mut s = 0i64;
            for k in 0..n {
                s += m[i][k] as i64 * residual[r * n + k] as i64;
            }
            tmp[r * n + i] = (s + add1) >> shift1;
        }
    }
    // Stage 2: transform columns (vertical).
    let mut out = vec![0i32; n * n];
    for c in 0..n {
        for i in 0..n {
            let mut s = 0i64;
            for k in 0..n {
                s += m[i][k] as i64 * tmp[k * n + c];
            }
            out[i * n + c] = ((s + add2) >> shift2) as i32;
        }
    }
    out
}

/// Inverse 2D transform of an `n×n` coefficient block (row-major) back to a
/// residual (row-major). Bit-exact to H.265 §8.6.4.2 for 8-bit samples
/// (`bdShift = 20 − BitDepth = 12`).
pub fn inverse(coeff: &[i32], n: usize, kind: TransformKind) -> Vec<i32> {
    let m = matrix(n, kind);
    let shift1 = 7i64;
    let shift2 = 20 - 8; // 12 for 8-bit
    let add1 = 1i64 << (shift1 - 1);
    let add2 = 1i64 << (shift2 - 1);

    // Stage 1: inverse transform columns (vertical), clip to 16-bit.
    let mut tmp = vec![0i64; n * n];
    for c in 0..n {
        for y in 0..n {
            let mut s = 0i64;
            for k in 0..n {
                s += m[k][y] as i64 * coeff[k * n + c] as i64;
            }
            tmp[y * n + c] = clip16((s + add1) >> shift1);
        }
    }
    // Stage 2: inverse transform rows (horizontal).
    let mut out = vec![0i32; n * n];
    for r in 0..n {
        for x in 0..n {
            let mut s = 0i64;
            for k in 0..n {
                s += m[k][x] as i64 * tmp[r * n + k];
            }
            out[r * n + x] = ((s + add2) >> shift2) as i32;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dct4_rows_orthogonal() {
        // Distinct DCT rows are orthogonal; a wrong table entry breaks this.
        for i in 0..4 {
            for j in 0..4 {
                let dot: i32 = (0..4).map(|k| DCT4[i][k] * DCT4[j][k]).sum();
                if i != j {
                    assert_eq!(dot, 0, "DCT4 rows {i},{j} not orthogonal");
                } else {
                    assert!(dot > 0);
                }
            }
        }
    }

    #[test]
    fn dct8_rows_near_orthogonal() {
        // The integer DCT is only *near*-orthogonal: same-parity row pairs can dot
        // to ±50 (a known artifact), but a transcription error would blow far past
        // that. Even↔odd pairs are exactly 0 by symmetry.
        for i in 0..8 {
            for j in 0..8 {
                let dot: i32 = (0..8).map(|k| DCT8[i][k] * DCT8[j][k]).sum();
                if i != j {
                    assert!(dot.abs() <= 64, "DCT8 rows {i},{j} dot={dot} too large");
                    if (i + j) % 2 == 1 {
                        assert_eq!(dot, 0, "opposite-parity DCT8 rows {i},{j} must be exact");
                    }
                } else {
                    assert!(dot > 30000, "DCT8 row {i} norm too small: {dot}");
                }
            }
        }
    }

    fn roundtrip(n: usize, kind: TransformKind, block: &[i32]) {
        let coeff = forward(block, n, kind);
        let recon = inverse(&coeff, n, kind);
        let mut max_err = 0;
        for k in 0..n * n {
            max_err = i32::max(max_err, (recon[k] - block[k]).abs());
        }
        assert!(
            max_err <= 2,
            "roundtrip err {max_err} for size {n} {kind:?}"
        );
    }

    #[test]
    fn dct_roundtrips() {
        // A ramp and an impulse recover through forward∘inverse (± rounding).
        let b4: Vec<i32> = (0..16).map(|i| (i * 7 % 40) - 20).collect();
        roundtrip(4, TransformKind::Dct, &b4);
        roundtrip(4, TransformKind::Dst, &b4);
        let b8: Vec<i32> = (0..64).map(|i| ((i * 13) % 60) - 30).collect();
        roundtrip(8, TransformKind::Dct, &b8);
        let b16: Vec<i32> = (0..256).map(|i| ((i * 17) % 80) - 40).collect();
        roundtrip(16, TransformKind::Dct, &b16);
    }

    #[test]
    fn dct16_nests_dct8() {
        // Nested design: DCT16 even rows (cols 0..8) equal DCT8 rows.
        for i in 0..8 {
            for j in 0..8 {
                assert_eq!(DCT16[2 * i][j], DCT8[i][j], "DCT16 nesting at {i},{j}");
            }
        }
    }

    #[test]
    fn dc_only_is_flat() {
        // A constant residual transforms to a single DC coefficient.
        let flat = vec![10i32; 16];
        let coeff = forward(&flat, 4, TransformKind::Dct);
        assert!(coeff[0].abs() > 0, "DC present");
        for (i, &c) in coeff.iter().enumerate().skip(1) {
            assert_eq!(c, 0, "non-DC coeff {i} should be ~0 for flat input");
        }
    }
}

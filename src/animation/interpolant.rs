#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    /// Snap to the previous keyframe's value.
    Step,
    /// Linear interpolation between two keyframes (slerp for quaternions).
    Linear,
    /// Cubic spline (not yet implemented — falls back to Linear).
    Cubic,
}

/// Find the (lower_index, alpha) sample for `t` in `times`.
/// Returns `lower_index = times.len()-1` and `alpha = 0` if `t >= last`.
pub(crate) fn find_segment(times: &[f32], t: f32) -> (usize, f32) {
    if times.is_empty() {
        return (0, 0.0);
    }
    if t <= times[0] {
        return (0, 0.0);
    }
    if t >= *times.last().unwrap() {
        return (times.len() - 1, 0.0);
    }
    // Binary search.
    let mut lo = 0usize;
    let mut hi = times.len() - 1;
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if times[mid] <= t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let t0 = times[lo];
    let t1 = times[hi];
    let alpha = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
    (lo, alpha)
}

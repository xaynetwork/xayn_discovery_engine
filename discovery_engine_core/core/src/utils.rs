use std::cmp::Ordering;

/// Allows comparing and sorting f32 even if `NaN` is involved.
///
/// Pretend that f32 has a total ordering.
///
/// `NaN` is treated as the lowest possible value if `nan_min`, similar to what [`f32::max`] does.
/// Otherwise it is treated as the highest possible value, similar to what [`f32::min`] does.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) fn nan_safe_f32_cmp_base(a: &f32, b: &f32, nan_min: bool) -> Ordering {
    a.partial_cmp(b).unwrap_or_else(|| {
        // if `partial_cmp` returns None we have at least one `NaN`,
        let cmp = match (a.is_nan(), b.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, _) => Ordering::Less,
            (_, true) => Ordering::Greater,
            _ => unreachable!("partial_cmp returned None but both numbers are not NaN"),
        };
        if nan_min {
            cmp
        } else {
            cmp.reverse()
        }
    })
}

/// Allows comparing and sorting f32 even if `NaN` is involved.
///
/// Pretend that f32 has a total ordering.
///
/// `NaN` is treated as the lowest possible value, similar to what [`f32::max`] does.
///
/// If this is used for sorting this will lead to an ascending order, like
/// for example `[NaN, 0.5, 1.5, 2.0]`.
///
/// By switching the input parameters around this can be used to create a
/// descending sorted order, like e.g.: `[2.0, 1.5, 0.5, NaN]`.
#[allow(clippy::trivially_copy_pass_by_ref)]
// we allow the lint because we may want to use the function for `std::slice::sort_by`
pub(crate) fn nan_safe_f32_cmp(a: &f32, b: &f32) -> Ordering {
    nan_safe_f32_cmp_base(a, b, true)
}

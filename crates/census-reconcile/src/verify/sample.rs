const MAX_SAMPLES: usize = 5000;

pub fn sample_indices(total_rows: usize, k: usize) -> Vec<usize> {
    if total_rows == 0 {
        return Vec::new();
    }
    let k = k.max(1);
    let natural = total_rows.checked_div(k).map_or(total_rows, |value| value);
    let count = natural.clamp(1, MAX_SAMPLES);
    let stride = if count > 1 {
        total_rows
            .checked_div(count)
            .map_or(total_rows, |value| value)
    } else {
        total_rows
    };
    (0..count)
        .map(|i| checked_mul(i, stride).map_or(usize::MAX, |value| value))
        .collect()
}

#[inline]
fn checked_mul(a: usize, b: usize) -> Option<usize> {
    a.checked_mul(b)
}

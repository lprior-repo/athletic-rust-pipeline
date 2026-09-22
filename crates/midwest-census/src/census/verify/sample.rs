//! Deterministic row sampling.
//!
//! Returns every k-th index from 0 up to `total_rows - 1`, capped at `MAX_SAMPLES`.
//! For a table of 12,345 rows with k=3 the result is indices 0, 3, 6, 9, …, 12,342 — but at
//! most 5,000 entries. The stride is `total_rows / samples` where `samples = min(total_rows / k,
//! MAX_SAMPLES)`, so the sampling density is proportional to k while never exceeding the cap.

/// Maximum rows we sample per sheet. A large workbook is verified by checking a fraction.
const MAX_SAMPLES: usize = 5000;

/// Return every k-th index from 0 up to `total_rows - 1`, capped at `MAX_SAMPLES`.
///
/// For a table of 12 345 rows with k = 3 the result is indices 0, 3, 6, 9, …, 12 342 — but at
/// most 5 000 entries. The stride is `total_rows / samples` where `samples = min(total_rows / k,
/// MAX_SAMPLES)`, so the sampling density is proportional to k while never exceeding the cap.
pub fn sample_indices(total_rows: usize, k: usize) -> Vec<usize> {
    if total_rows == 0 {
        return Vec::new();
    }
    let k = k.max(1);
    let natural = total_rows.checked_div(k).unwrap_or(total_rows);
    let count = natural.clamp(1, MAX_SAMPLES);
    let stride = if count > 1 {
        total_rows.checked_div(count).unwrap_or(total_rows)
    } else {
        total_rows
    };
    (0..count)
        .map(|i| checked_mul(i, stride).unwrap_or(usize::MAX))
        .collect()
}

/// Multiply two usize values with overflow protection.
///
/// Overflow in the index computation is unreachable with the bounds enforced
/// by `count` and `MAX_SAMPLES`, but the checker cannot prove that.
#[inline]
fn checked_mul(a: usize, b: usize) -> Option<usize> {
    a.checked_mul(b)
}

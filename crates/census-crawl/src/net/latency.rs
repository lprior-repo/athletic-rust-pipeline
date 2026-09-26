//! The transport-latency histogram behind §45's `average latency`, `p50`, `p95` and `p99`.
//!
//! One bucket per edge, so the memory a run spends on latency is a fixed twenty counters rather than
//! a sample list that grows with traffic — the census fetches for days, and a per-request sample
//! vector would be the only unbounded structure in the client. The price is resolution: a percentile
//! is the edge of the bucket that covers it, an upper bound at that resolution rather than a claim
//! about one request.
//!
//! Split out of `types.rs` for the file budget; the methods are the same ones, and `FetchStats`
//! keeps its public surface because an inherent `impl` block adds to the type wherever it lives.

use super::types::FetchStats;

/// Upper edges of the latency histogram's buckets, in milliseconds, ascending.
///
/// A percentile is reported as the edge of the bucket that covers it, so the number the sheet prints
/// is an upper bound at the histogram's resolution rather than a claim about one request. The last
/// edge is the request timeout: a slow-but-answered request cannot exceed it, and a bucket past it
/// would only ever hold the wait for a timeout to fire.
pub const LATENCY_BUCKET_EDGES_MS: [u64; 20] = [
    5, 10, 20, 30, 50, 75, 100, 150, 250, 400, 600, 900, 1500, 2500, 4000, 6000, 10000, 20000,
    30000, 45000,
];

/// One bucket per edge: bucket `i` holds the requests that took more than edge `i-1` and at most
/// edge `i`, and bucket `0` holds everything at or under the first edge.
pub const LATENCY_BUCKET_COUNT: usize = LATENCY_BUCKET_EDGES_MS.len();

impl FetchStats {
    /// Record one measured transport latency.
    ///
    /// Saturating throughout: a counter that wrapped would understate the run's traffic, which is
    /// the one thing these numbers exist to state.
    pub fn record_latency(&mut self, millis: u64) {
        self.latency_ms_sum = self.latency_ms_sum.saturating_add(millis);
        self.latency_ms_max = self.latency_ms_max.max(millis);
        let bucket = match LATENCY_BUCKET_EDGES_MS
            .iter()
            .position(|edge| millis <= *edge)
        {
            Some(index) => index,
            None => LATENCY_BUCKET_COUNT.saturating_sub(1),
        };
        if let Some(slot) = self.latency_buckets.get_mut(bucket) {
            *slot = slot.saturating_add(1);
        }
    }

    /// How many latencies were measured.
    pub fn latency_samples(&self) -> u64 {
        self.latency_buckets
            .iter()
            .copied()
            .fold(0_u64, u64::saturating_add)
    }

    /// The mean transport latency in milliseconds, when anything was measured.
    pub fn latency_ms_avg(&self) -> Option<u64> {
        let samples = self.latency_samples();
        if samples == 0 {
            return None;
        }
        self.latency_ms_sum.checked_div(samples)
    }

    /// The bucket edge covering `percentile` of the measured latencies, when anything was measured.
    ///
    /// An upper bound: the request at that rank took no longer than the edge returned, and the
    /// histogram cannot say by how much less. `None` where nothing was measured, because a
    /// percentile of no samples is not zero.
    pub fn latency_percentile_ms(&self, percentile: u8) -> Option<u64> {
        let samples = self.latency_samples();
        if samples == 0 {
            return None;
        }
        let percentile = u64::from(percentile.min(100));
        let rank = samples
            .saturating_mul(percentile)
            .saturating_add(99)
            .saturating_div(100)
            .max(1);
        let mut seen = 0_u64;
        for (index, count) in self.latency_buckets.iter().enumerate() {
            seen = seen.saturating_add(*count);
            if seen >= rank {
                return LATENCY_BUCKET_EDGES_MS.get(index).copied();
            }
        }
        LATENCY_BUCKET_EDGES_MS.last().copied()
    }
}

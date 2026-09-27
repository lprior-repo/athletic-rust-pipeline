
use super::types::FetchStats;

pub const LATENCY_BUCKET_EDGES_MS: [u64; 20] = [
    5, 10, 20, 30, 50, 75, 100, 150, 250, 400, 600, 900, 1500, 2500, 4000, 6000, 10000, 20000,
    30000, 45000,
];

pub const LATENCY_BUCKET_COUNT: usize = LATENCY_BUCKET_EDGES_MS.len();

impl FetchStats {
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

    pub fn latency_samples(&self) -> u64 {
        self.latency_buckets
            .iter()
            .copied()
            .fold(0_u64, u64::saturating_add)
    }

    pub fn latency_ms_avg(&self) -> Option<u64> {
        let samples = self.latency_samples();
        if samples == 0 {
            return None;
        }
        self.latency_ms_sum.checked_div(samples)
    }

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

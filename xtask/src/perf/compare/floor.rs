pub(super) const TAIL_JITTER_FLOOR: f64 = 0.2;
const PEAK_RSS_JITTER_FLOOR_KIB: u64 = 1024;
const PEAK_RSS_JITTER_CEILING_DIVISOR: u64 = 5;

pub(super) fn case_timing_floor(wall_time: f64, tail_time: f64) -> f64 {
    if !wall_time.is_finite()
        || wall_time <= 0.0
        || !tail_time.is_finite()
        || tail_time <= wall_time
    {
        return 0.0;
    }
    ((tail_time - wall_time) / wall_time).min(TAIL_JITTER_FLOOR)
}

pub(super) fn peak_rss_floor(recorded_kib: u64) -> u64 {
    recorded_kib
        .checked_div(PEAK_RSS_JITTER_CEILING_DIVISOR)
        .map_or(PEAK_RSS_JITTER_FLOOR_KIB, |share| {
            PEAK_RSS_JITTER_FLOOR_KIB.max(share)
        })
}

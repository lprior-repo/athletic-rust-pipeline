use indexmap::IndexMap;

pub(crate) fn bump(slot: &mut usize, by: usize) {
    *slot = slot.saturating_add(by);
}

pub(crate) fn tally(map: &mut IndexMap<String, usize>, key: &str, by: usize) {
    bump(map.entry(key.to_string()).or_insert(0), by);
}

pub(crate) fn count_len(count: i64) -> usize {
    usize::try_from(count).map_or(usize::MAX, core::convert::identity)
}

pub(crate) fn len_count(len: usize) -> i64 {
    i64::try_from(len).map_or(i64::MAX, core::convert::identity)
}

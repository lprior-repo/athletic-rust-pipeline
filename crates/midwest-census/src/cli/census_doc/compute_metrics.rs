//! Compute Co2027 and recruiting metrics from CSV rows.

use std::collections::HashMap;

/// Metrics derived from the Co2027 and recruiting CSVs.
pub(crate) struct Metrics {
    pub(super) with_an: usize,
    pub(super) with_ms: usize,
    pub(super) multi_rows: usize,
    pub(super) rec_coach: usize,
    pub(super) rec_email: usize,
    pub(super) rec_ad: usize,
}

pub(super) fn compute(
    co2027: &[HashMap<String, String>],
    recruiting: &[HashMap<String, String>],
) -> Metrics {
    let with_an = co2027
        .iter()
        .filter(|row| {
            row.get("athleticnet_athlete_id")
                .is_some_and(|id| !id.is_empty())
        })
        .count();
    let with_ms = co2027
        .iter()
        .filter(|row| {
            row.get("milesplit_athlete_id")
                .is_some_and(|id| !id.is_empty())
        })
        .count();
    let multi_rows = co2027
        .iter()
        .filter(|row| {
            row.get("source_namespaces")
                .is_some_and(|ns| ns.contains(';'))
        })
        .count();

    let rec_coach = recruiting
        .iter()
        .filter(|row| {
            row.get("head_track_coach").is_some_and(|c| !c.is_empty())
                || row.get("head_xc_coach").is_some_and(|c| !c.is_empty())
        })
        .count();
    let rec_email = recruiting
        .iter()
        .filter(|row| {
            row.get("head_track_coach_email")
                .is_some_and(|e| !e.is_empty())
                || row
                    .get("head_xc_coach_email")
                    .is_some_and(|e| !e.is_empty())
        })
        .count();
    let rec_ad = recruiting
        .iter()
        .filter(|row| {
            row.get("athletic_director_email")
                .is_some_and(|e| !e.is_empty())
        })
        .count();

    Metrics {
        with_an,
        with_ms,
        multi_rows,
        rec_coach,
        rec_email,
        rec_ad,
    }
}

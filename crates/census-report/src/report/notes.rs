use super::rows::RowCounts;
use super::{Scope, StateCensus, NON_CORE_SOURCE_IDS};
use census_domain::JurisdictionBucket;
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn bump(counter: &mut usize) {
    *counter = counter.saturating_add(1);
}

pub(super) fn add(total: &mut usize, value: usize) {
    *total = total.saturating_add(value);
}

pub(super) fn state_entry(
    by_state: &mut BTreeMap<JurisdictionBucket, StateCensus>,
    state: JurisdictionBucket,
) -> &mut StateCensus {
    by_state.entry(state).or_insert_with(|| StateCensus {
        state: state.into(),
        ..StateCensus::default()
    })
}

pub(super) fn census_notes(
    scope: Scope,
    counts: &RowCounts,
    coach_sources_empty: bool,
    out: &Path,
) -> Vec<String> {
    let mut notes = vec![format!(
        "scope={} schools={} athletes={} coaches={} from {}",
        scope.as_str(),
        counts.schools,
        counts.athletes,
        counts.coaches,
        out.display()
    )];
    append_scope_notes(&mut notes, scope, counts);
    append_population_notes(&mut notes, counts, coach_sources_empty);
    notes
}

fn append_scope_notes(notes: &mut Vec<String>, scope: Scope, counts: &RowCounts) {
    if scope == Scope::Core {
        notes.push(format!(
            "core scope drops {} rows whose only evidence is {}",
            counts.dropped,
            NON_CORE_SOURCE_IDS.join("/")
        ));
        notes.push(
            "core performance publication keeps a row only when at least one core-evidence source remains; non-core-only rows are omitted from core counts and sheets"
                .to_string(),
        );
    }
}

fn append_population_notes(notes: &mut Vec<String>, counts: &RowCounts, coach_sources_empty: bool) {
    if counts.athletes == 0 {
        notes.push(
            "no athletes consolidated yet — run `collect` then `consolidate` before `report`"
                .to_string(),
        );
    }
    if counts.coaches > 0 && coach_sources_empty {
        notes.push("coaches present but no coach source identities recorded".to_string());
    }
}

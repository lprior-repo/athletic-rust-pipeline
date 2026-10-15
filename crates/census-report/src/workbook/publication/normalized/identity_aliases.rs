use super::{encode, write_csv};
use crate::report::{Derivation, ReportResult};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const HEADER: [&str; 12] = [
    "original_id",
    "canonical_id",
    "entity_type",
    "resolution_state",
    "successor_ids",
    "effective_revision",
    "resolution_reference",
    "input_generation",
    "publication_membership",
    "historical_state_status",
    "resolved_id",
    "resolution_reference_status",
];

pub(super) fn write(directory: &Path, derivation: &Derivation<'_>) -> ReportResult<u64> {
    let dataset = derivation.dataset();
    let projection = dataset.identities();
    let published: BTreeSet<_> = derivation
        .athletes()
        .iter()
        .map(|athlete| athlete.id.as_str())
        .collect();
    let mut transitions: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for transition in projection.alias_transitions() {
        transitions
            .entry(transition.subject())
            .or_default()
            .insert(transition.decision_id());
    }
    let empty = BTreeSet::new();
    write_csv(&directory.join("identity_aliases.csv"), &HEADER, |writer| {
        for athlete in &dataset.athletes {
            let subject = athlete.id.as_str();
            let canonical = projection.canonical_id(subject);
            let included = published.contains(canonical);
            let state = projection
                .status(subject)
                .map_err(census_store::StoreError::from)?;
            let references = transitions
                .get(subject)
                .map_or(&empty, core::convert::identity);
            writer.push([
                subject.into(),
                if included { canonical } else { "unavailable" }.into(),
                "athlete".into(),
                state.as_str().into(),
                "unavailable".into(),
                "unavailable".into(),
                encode(&references)?,
                dataset.lineage.input_generation.clone(),
                if included {
                    "canonical_in_publication"
                } else {
                    "canonical_outside_publication"
                }
                .into(),
                "effective_revision_split_successors_supersession_not_retained".into(),
                canonical.into(),
                if references.is_empty() {
                    "no_retained_alias_transition"
                } else {
                    "retained_projection_transitions"
                }
                .into(),
            ])?;
        }
        Ok(())
    })
}

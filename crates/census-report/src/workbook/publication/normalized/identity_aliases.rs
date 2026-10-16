use super::{encode, write_csv};
use crate::export::public_identity::{
    output, population::IdentityPopulation, VerifiedPublicIdentityView,
};
use crate::report::{Derivation, ReportResult};
use crate::workbook::public_identity;
use census_domain::model::AthleteIdentityProjection;
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
    let view = public_identity::view(dataset)?;
    let identities = IdentityPopulation::of(dataset)?;
    let headers = headers(view);
    let published: BTreeSet<_> = derivation
        .athletes()
        .iter()
        .map(|athlete| athlete.id.as_str())
        .collect();
    let transitions = transition_references(&projection);
    let empty = BTreeSet::new();
    write_csv(
        &directory.join("identity_aliases.csv"),
        &headers,
        |writer| {
            dataset.athletes.iter().try_for_each(|athlete| {
                let subject = athlete.id.as_str();
                let canonical = identities.canonical_candidate(subject).ok_or_else(|| {
                    super::defect("alias absent from publication identity authority")
                })?;
                let included = published.contains(canonical);
                let state = projection
                    .status(subject)
                    .map_err(census_store::StoreError::from)?;
                let references = transitions
                    .get(subject)
                    .map_or(&empty, core::convert::identity);
                let fields = [
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
                ];
                writer.push_vec(fields.into_iter().chain(audit(view, subject)?).collect())
            })
        },
    )
}

fn headers(view: Option<&VerifiedPublicIdentityView>) -> Vec<&'static str> {
    HEADER
        .into_iter()
        .map(|header| match (view, header) {
            (Some(_), "resolution_state") => "historical_automatic_identity_status",
            _ => header,
        })
        .chain(view.into_iter().flat_map(|_| {
            [
                "public_athlete_id",
                "publication_identity_status",
                "identity_resolution",
            ]
        }))
        .collect()
}

fn transition_references(projection: &AthleteIdentityProjection) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut transitions: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for transition in projection.alias_transitions() {
        transitions
            .entry(transition.subject())
            .or_default()
            .insert(transition.decision_id());
    }
    transitions
}

fn audit(
    view: Option<&VerifiedPublicIdentityView>,
    candidate: &str,
) -> ReportResult<impl Iterator<Item = String> + use<>> {
    let fields = match view {
        None => None,
        Some(view) => {
            let group = view
                .group(candidate)
                .ok_or_else(|| super::defect("alias absent from retained public identity"))?;
            let resolution = view.candidate_resolution(candidate).ok_or_else(|| {
                super::defect("alias resolution absent from retained public identity")
            })?;
            Some([
                group
                    .public_id()
                    .map_or_else(String::new, |id| id.as_str().to_owned()),
                output::status(group, resolution).into(),
                encode(&output::ResolutionAudit(resolution))?,
            ])
        }
    };
    Ok(fields.into_iter().flatten())
}

#[cfg(test)]
mod tests;

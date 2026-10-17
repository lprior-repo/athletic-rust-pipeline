use super::*;
use census_domain::public_identity_history::{
    BindingClaim, BindingResolution, CandidateDecisionReference, HistoryRequest, PublicResolution,
};

#[derive(Debug, Clone, serde::Serialize)]
pub(super) enum PersonJoin {
    Accepted {
        athlete: AthleteId,
        decision: String,
        binding: String,
    },
    Unresolved {
        connected: Vec<AthleteId>,
        reason: JoinGap,
    },
}

pub(super) type PersonIndex = BTreeMap<String, Candidates>;
pub(super) type PhysicalPeople = BTreeMap<String, BTreeSet<AthleteId>>;

#[derive(Default, serde::Serialize)]
pub(super) struct Candidates {
    accepted: BTreeMap<String, (AthleteId, String)>,
    connected: BTreeSet<AthleteId>,
}

pub(super) fn index(
    dataset: &ExportDataset,
    physical: &PhysicalPeople,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<PersonIndex> {
    let view = dataset.verified_public_identities().map_err(report_store)?;
    let accepted = view.accepted_snapshot();
    let mut index = BTreeMap::<String, Candidates>::new();
    for (person, subjects) in physical {
        encoding::admit(&(person, subjects), grant)?;
        index
            .entry(person.clone())
            .or_default()
            .connected
            .extend(subjects.iter().cloned());
    }
    for record in accepted.history_records() {
        encoding::admit(&record.request, grant)?;
        for (binding, decision) in bindings(&record.request) {
            let person = checked(SourceSubject::new(
                binding.source.namespace.clone(),
                binding.source.identifier.clone(),
            ))?;
            let family = index.entry(person.logical_id().into()).or_default();
            let Some(group) = view.group(binding.candidate.as_str()) else {
                continue;
            };
            family.connected.insert(group.canonical_candidate().cast());
            let BindingResolution::Resolved {
                identity,
                superseded_by: None,
                ..
            } = accepted.resolve_binding(&binding.id)?
            else {
                continue;
            };
            let public = match identity {
                PublicResolution::Canonical(value)
                | PublicResolution::Redirect {
                    successor: value, ..
                }
                | PublicResolution::RevokedRedirect {
                    successor: value, ..
                } => value.public_id,
                PublicResolution::Unknown
                | PublicResolution::Unresolved
                | PublicResolution::Ambiguous { .. } => continue,
            };
            if group.public_id() == Some(&public) {
                family.accepted.insert(
                    binding.id.clone(),
                    (
                        group.canonical_candidate().cast(),
                        decision.evidence_digest.clone(),
                    ),
                );
            }
        }
    }
    Ok(index)
}

#[derive(Debug, Clone, serde::Serialize)]
pub(super) enum JoinGap {
    MissingAcceptedBinding,
    ConflictingCandidateOwnership,
}

pub(super) fn join(
    index: &PersonIndex,
    person: &SourceSubject,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<PersonJoin> {
    let Some(family) = index.get(person.logical_id()) else {
        return Ok(PersonJoin::Unresolved {
            connected: Vec::new(),
            reason: JoinGap::MissingAcceptedBinding,
        });
    };
    encoding::admit(family, grant)?;
    let connected = family.connected.iter().cloned().collect();
    if family.connected.len() != 1 {
        return Ok(PersonJoin::Unresolved {
            connected,
            reason: JoinGap::ConflictingCandidateOwnership,
        });
    }
    if let Some((binding, (athlete, decision))) = family.accepted.iter().next() {
        return Ok(PersonJoin::Accepted {
            athlete: athlete.clone(),
            decision: decision.clone(),
            binding: binding.clone(),
        });
    }
    Ok(PersonJoin::Unresolved {
        connected,
        reason: JoinGap::MissingAcceptedBinding,
    })
}

fn bindings(request: &HistoryRequest) -> Vec<(&BindingClaim, &CandidateDecisionReference)> {
    match request {
        HistoryRequest::Establish { binding, evidence }
        | HistoryRequest::Bind {
            binding, evidence, ..
        } => vec![(binding, evidence)],
        HistoryRequest::EstablishMatchedGroup { bindings, evidence } => {
            bindings.iter().map(|binding| (binding, evidence)).collect()
        }
        HistoryRequest::BindCandidateMembership { .. }
        | HistoryRequest::Merge { .. }
        | HistoryRequest::Split(_)
        | HistoryRequest::Revoke(_) => Vec::new(),
    }
}

impl PersonJoin {
    pub(super) fn subjects(&self) -> &[AthleteId] {
        match self {
            Self::Accepted { athlete, .. } => std::slice::from_ref(athlete),
            Self::Unresolved { connected, .. } => connected,
        }
    }
    pub(super) fn connected(&self, athlete: &AthleteId) -> bool {
        match self {
            Self::Accepted { athlete: bound, .. } => bound == athlete,
            Self::Unresolved { connected, .. } => connected.contains(athlete),
        }
    }
}

pub(super) fn physical_people(
    dataset: &ExportDataset,
    bound: &BoundRosterSourceViewV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<PhysicalPeople> {
    let view = dataset.verified_public_identities()?;
    let mut people = BTreeMap::<String, BTreeSet<AthleteId>>::new();
    bound.visit_athletes(grant, |athlete, grant| {
        let group = view
            .group(athlete.id.as_str())
            .ok_or_else(|| invalid("physical source person lacks identity group"))?;
        for source in athlete.identities() {
            let person = checked(SourceSubject::new(
                source.namespace.clone(),
                source.id.clone(),
            ))?;
            charge(
                grant,
                person
                    .logical_id()
                    .len()
                    .checked_add(athlete.id.as_str().len())
                    .ok_or(StoreError::CounterOverflow)?,
                1,
            )?;
            people
                .entry(person.logical_id().into())
                .or_default()
                .insert(group.canonical_candidate().cast());
        }
        Ok(())
    })?;
    Ok(people)
}

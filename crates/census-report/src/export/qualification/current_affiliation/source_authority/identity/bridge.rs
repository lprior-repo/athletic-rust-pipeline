use super::*;
use census_domain::model::{
    athlete_identity_digest, AppliedAthleteIdentity, AppliedIdentityKind, CanonicalAthlete,
};
use census_domain::public_identity_history::{CandidateEvidenceKind, ReplayLimits};

type View = crate::export::public_identity::VerifiedPublicIdentityView;
type Decisions<'a> = BTreeMap<&'a str, Option<&'a AppliedAthleteIdentity>>;

pub(super) fn extend(
    dataset: &ExportDataset,
    index: &mut PersonIndex,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let view = dataset.verified_public_identities().map_err(report_store)?;
    let active = active_decisions(dataset, view, grant)?;
    charge(grant, 0, dataset.athletes.len())?;
    for athlete in &dataset.athletes {
        let Some(decision) = active.get(athlete.id.as_str()) else {
            continue;
        };
        charge(grant, 0, athlete.source_links.len())?;
        for source in &athlete.source_links {
            if source.namespace != census_domain::model::SourceNamespace::MilesplitAthlete
                || athlete.source.as_ref().is_some_and(|primary| {
                    primary.namespace == source.namespace && primary.id == source.id
                })
            {
                continue;
            }
            let Some((binding, canonical)) = primary(index, view, athlete, grant)? else {
                continue;
            };
            let person = checked(SourceSubject::new(
                source.namespace.clone(),
                source.id.clone(),
            ))?;
            encoding::admit(&(&person, &athlete.id, &binding, decision), grant)?;
            let family = index.entry(person.logical_id().into()).or_default();
            family
                .accepted
                .insert(binding, (canonical, decision.clone()));
            family.secondary_candidates.insert(athlete.id.clone());
        }
    }
    Ok(())
}

fn primary(
    index: &PersonIndex,
    view: &View,
    athlete: &CanonicalAthlete,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<(String, AthleteId)>> {
    let Some(source) = &athlete.source else {
        return Ok(None);
    };
    if athlete.id
        != CanonicalAthlete::mint(
            &athlete.school,
            &athlete.canonical_name,
            athlete.grad_year,
            athlete.gender,
            source,
        )
    {
        return Ok(None);
    }
    let person = checked(SourceSubject::new(
        source.namespace.clone(),
        source.id.clone(),
    ))?;
    let Some(family) = index.get(person.logical_id()) else {
        return Ok(None);
    };
    if family.connected.len() != 1 {
        return Ok(None);
    }
    charge(grant, 0, family.accepted.len())?;
    let mut matches = family.accepted.iter().filter(|(id, _)| {
        view.accepted_snapshot()
            .binding_claim(id)
            .is_some_and(|binding| {
                binding.candidate.as_str() == athlete.id.as_str()
                    && binding.source.namespace == source.namespace
                    && binding.source.identifier == source.id
            })
    });
    let Some((binding, (canonical, _))) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Ok(None);
    }
    Ok(Some((binding.clone(), canonical.clone())))
}

fn active_decisions(
    dataset: &ExportDataset,
    view: &View,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<BTreeMap<String, String>> {
    let decisions = decisions(dataset, grant)?;
    let digests = digests(dataset, grant)?;
    let records = view.accepted_snapshot().history_records();
    charge(grant, 0, records.len())?;
    let mut active = BTreeMap::new();
    for record in records {
        let reference = reference(&record.request);
        charge(grant, 0, reference.members.len())?;
        invalidate(&record.request, &mut active, grant)?;
        if correction(&record.request, reference) {
            continue;
        }
        let Some(decision) = decisions
            .get(reference.application_id.as_str())
            .copied()
            .flatten()
        else {
            continue;
        };
        if !matches!(
            decision.kind,
            AppliedIdentityKind::SourceBound | AppliedIdentityKind::SamePerson
        ) {
            continue;
        }
        encoding::admit(reference, grant)?;
        charge(
            grant,
            0,
            decision
                .members
                .len()
                .checked_mul(4)
                .ok_or(StoreError::CounterOverflow)?,
        )?;
        if !checked(reference.matches_applied(decision, ReplayLimits::SMALL))?
            || !same_group(view, reference)
        {
            continue;
        }
        for member in &decision.members {
            if digests.get(member.subject.as_str()) == Some(&member.evidence_digest) {
                active.insert(
                    member.subject.to_string(),
                    reference.evidence_digest.clone(),
                );
            }
        }
    }
    Ok(active)
}

fn correction(request: &HistoryRequest, reference: &CandidateDecisionReference) -> bool {
    reference.kind == CandidateEvidenceKind::Correction
        || matches!(
            request,
            HistoryRequest::Split(_) | HistoryRequest::Revoke(_)
        )
}

fn digests<'a>(
    dataset: &'a ExportDataset,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<BTreeMap<&'a str, String>> {
    charge(grant, 0, dataset.athletes.len())?;
    let mut digests = BTreeMap::new();
    for athlete in &dataset.athletes {
        encoding::admit(athlete, grant)?;
        digests.insert(athlete.id.as_str(), athlete_identity_digest(athlete)?);
    }
    Ok(digests)
}

fn decisions<'a>(
    dataset: &'a ExportDataset,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Decisions<'a>> {
    charge(grant, 0, dataset.identity_decisions.len())?;
    let mut decisions = BTreeMap::new();
    for decision in &dataset.identity_decisions {
        charge(grant, 0, decision.members.len())?;
        encoding::admit(decision, grant)?;
        decisions
            .entry(decision.id.as_str())
            .and_modify(|value| *value = None)
            .or_insert(Some(decision));
    }
    Ok(decisions)
}

fn reference(request: &HistoryRequest) -> &CandidateDecisionReference {
    match request {
        HistoryRequest::Establish { evidence, .. }
        | HistoryRequest::EstablishMatchedGroup { evidence, .. }
        | HistoryRequest::Bind { evidence, .. }
        | HistoryRequest::BindCandidateMembership { evidence, .. }
        | HistoryRequest::Merge { evidence, .. } => evidence,
        HistoryRequest::Split(request) => &request.evidence,
        HistoryRequest::Revoke(request) => &request.evidence,
    }
}

fn invalidate(
    request: &HistoryRequest,
    active: &mut BTreeMap<String, String>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    for member in &reference(request).members {
        active.remove(member.as_str());
    }
    let affected = match request {
        HistoryRequest::Split(request) => request.affected_record_keys.as_slice(),
        HistoryRequest::Revoke(request) => request.affected_record_keys.as_slice(),
        _ => &[],
    };
    charge(grant, 0, affected.len())?;
    for key in affected {
        if let Some(candidate) = key.strip_prefix("athletes:") {
            active.remove(candidate);
        }
    }
    Ok(())
}

fn same_group(view: &View, reference: &CandidateDecisionReference) -> bool {
    let Some(first) = reference
        .members
        .first()
        .and_then(|candidate| view.public_id(candidate.as_str()))
    else {
        return false;
    };
    reference
        .members
        .iter()
        .all(|candidate| view.public_id(candidate.as_str()) == Some(first))
}

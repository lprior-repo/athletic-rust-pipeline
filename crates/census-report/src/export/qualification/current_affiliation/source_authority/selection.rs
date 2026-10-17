use super::*;
use census_domain::affiliation_candidates::current::{
    assess_current_affiliation, CurrentAffiliationInventory, CurrentAffiliationOutcome,
    CurrentAffiliationRequest,
};
use census_domain::model::Sport;
use chrono::{DateTime, Utc};
use std::num::NonZeroUsize;

mod assertions;
mod fact;
mod projection;
use assertions::{assertions, Inputs};
use projection::{merge, retain};

pub(super) fn assess(
    dataset: &ExportDataset,
    profile: &QueryProfileV1,
    captures: &[material::Capture],
    physical: &physical::PhysicalEvidence,
    known: DeclaredFrozenView<'_>,
    cutoff: DateTime<Utc>,
    audit: &mut RetainedAudit,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<BTreeMap<AthleteId, Resolution>> {
    let subjects = subjects(dataset)?;
    let evidence = captures
        .iter()
        .map(material::Capture::evidence)
        .collect::<StoreResult<Vec<_>>>()?;
    let published = captures
        .iter()
        .map(|capture| match &capture.owner_span {
            Ok(spans) => vec![capture.proposal.scope().owner(); spans.len()],
            Err(_) => Vec::new(),
        })
        .collect::<Vec<_>>();
    let families = families(captures, grant)?;
    let missing: BTreeSet<_> = physical.gaps.iter().map(|gap| &gap.athlete).collect();
    let bounds = checked(KnownBounds::new(
        known.lineage,
        known.revision,
        known.revision,
    ))?;
    let mut rows = BTreeMap::new();
    for athlete in subjects {
        charge(grant, 0, 1)?;
        let mut resolution = Resolution::Unknown;
        let mut unsupported = missing.contains(&athlete);
        for sport in &profile.input().sports {
            let members = families.get(&athlete).map_or(&[][..], Vec::as_slice);
            let inputs = assertions(
                &athlete,
                *sport,
                members,
                &physical.supported,
                captures,
                &evidence,
                &published,
                bounds,
                audit,
                grant,
            )?;
            unsupported |= inputs.gapped;
            let request = CurrentAffiliationRequest {
                athlete: &athlete,
                profile,
                sport: *sport,
                known,
                acquisition_cutoff: cutoff,
                grant: selection_grant()?,
            };
            unsupported |= select(request, &inputs, &mut resolution, audit, grant)?;
        }
        if unsupported {
            resolution.withhold();
        }
        rows.insert(athlete, resolution);
    }
    Ok(rows)
}

fn selection_grant() -> StoreResult<SelectionGrant> {
    let nonzero =
        |value| NonZeroUsize::new(value).ok_or_else(|| invalid("zero selector resource grant"));
    checked(SelectionGrant::new(
        nonzero(256)?,
        nonzero(512)?,
        nonzero(768)?,
        nonzero(8_388_608)?,
        nonzero(2_000_000)?,
    ))
}

fn families(
    captures: &[material::Capture],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<BTreeMap<AthleteId, Vec<(usize, usize)>>> {
    let mut families = BTreeMap::<AthleteId, Vec<(usize, usize)>>::new();
    for (capture_index, capture) in captures.iter().enumerate() {
        for (member_index, member) in capture.members.iter().enumerate() {
            for subject in member.join.subjects() {
                charge(
                    grant,
                    subject
                        .as_str()
                        .len()
                        .checked_add(std::mem::size_of::<(usize, usize)>())
                        .ok_or(StoreError::CounterOverflow)?,
                    1,
                )?;
                families
                    .entry(subject.clone())
                    .or_default()
                    .push((capture_index, member_index));
            }
        }
    }
    Ok(families)
}

fn select<'a>(
    request: CurrentAffiliationRequest<'a>,
    inputs: &'a Inputs<'a>,
    resolution: &mut Resolution,
    audit: &mut RetainedAudit,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<bool> {
    let count = inputs.facts.len();
    let work = count
        .checked_mul(count)
        .and_then(|pairs| pairs.checked_mul(5))
        .ok_or(StoreError::CounterOverflow)?;
    charge(grant, 0, work)?;
    let selected = checked(assess_current_affiliation(
        request,
        CurrentAffiliationInventory {
            declared_view: request.known,
            assertions: &inputs.facts,
            changes: &[],
        },
    ))?;
    let unsupported = matches!(
        selected.outcome,
        CurrentAffiliationOutcome::SubjectUnresolved
            | CurrentAffiliationOutcome::EndedWithoutReplacement
            | CurrentAffiliationOutcome::NonSchool(_)
    );
    if unsupported {
        audit.reference(
            request.athlete,
            &format!(
                "roster_selector_outcome:{:?}:{:?}",
                request.sport, selected.outcome
            ),
        )?;
        audit.gap(request.athlete.clone(), Gap::IdentityUnaccepted)?;
    }
    retain(request.athlete, request.sport, &selected.facts, audit)?;
    merge(resolution, selected.outcome, &selected.facts);
    Ok(unsupported)
}

fn subjects(dataset: &ExportDataset) -> StoreResult<BTreeSet<AthleteId>> {
    let view = dataset.verified_public_identities().map_err(report_store)?;
    Ok(dataset
        .athletes
        .iter()
        .filter_map(|athlete| {
            view.group(athlete.id.as_str())
                .map(|group| group.canonical_candidate().cast())
        })
        .collect())
}

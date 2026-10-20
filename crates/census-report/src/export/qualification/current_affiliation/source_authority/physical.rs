use super::super::note;
use super::*;
use crate::export::qualification::physical_observations::OBSERVATION_BUDGET;
use census_domain::model::CanonicalAthlete;
use census_domain::roster_affiliation_authority::RosterMemberObservationV1;
#[derive(Clone, Copy)]
enum PersonOwnership {
    PrimaryOnly,
    AcceptedSecondary,
}

#[derive(Debug, serde::Serialize)]
pub(super) struct PhysicalGap {
    pub(super) athlete: AthleteId,
    reference: String,
    reason: &'static str,
}

#[derive(Debug, serde::Serialize)]
pub(super) struct PhysicalEvidence {
    pub(super) gaps: Vec<PhysicalGap>,
    pub(super) supported: BTreeSet<String>,
    witnesses: Vec<PhysicalWitness>,
}

pub(super) fn read(
    dataset: &ExportDataset,
    bound: &BoundRosterSourceViewV1<'_>,
    captures: &[material::Capture],
    audit: &mut RetainedAudit,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<PhysicalEvidence> {
    let view = dataset.verified_public_identities()?;
    let profile = dataset.selected_authority()?.profile();
    census_domain::profile_execution::validate_retained_selected_qualification_profile(
        profile,
        dataset.semantic_policy()?,
    )
    .map_err(|error| invalid(&error.to_string()))?;
    let ownership = if profile.input().policies.affiliation.revision().get() == 6 {
        PersonOwnership::AcceptedSecondary
    } else {
        PersonOwnership::PrimaryOnly
    };
    let indexed = index(captures, grant)?;
    let mut result = PhysicalEvidence {
        gaps: Vec::new(),
        supported: BTreeSet::new(),
        witnesses: Vec::new(),
    };
    bound.visit_athletes(grant, |athlete, grant| {
        let group = view
            .group(athlete.id.as_str())
            .ok_or_else(|| invalid("physical roster subject absent from identity inventory"))?;
        let canonical = group.canonical_candidate().cast();
        observe(
            &mut result,
            &indexed,
            ownership,
            &athlete,
            &canonical,
            grant,
            audit,
        )
    })?;
    Ok(result)
}
fn observe(
    result: &mut PhysicalEvidence,
    indexed: &Rows<'_>,
    ownership: PersonOwnership,
    athlete: &CanonicalAthlete,
    canonical: &AthleteId,
    grant: &mut AttachmentReplayGrantV1,
    audit: &mut RetainedAudit,
) -> StoreResult<()> {
    let observation_digest = census_domain::model::serialized_digest(athlete)?;
    for evidence in &athlete.evidence {
        charge(grant, 0, 1)?;
        let Some(note) = note::read(evidence)? else {
            continue;
        };
        audit.reference(canonical, note.source_row_key())?;
        let members = indexed
            .get(note.source_row_key())
            .map_or(&[][..], Vec::as_slice);
        let mut witnessed = false;
        for (capture, member) in members {
            if matches!(ownership, PersonOwnership::AcceptedSecondary) {
                charge(grant, 0, athlete.source_links.len())?;
            }
            matching(capture, member, athlete, &note, ownership)?;
            if !member.join.connected(canonical) {
                continue;
            }
            witnessed = true;
            let witness = PhysicalWitness {
                member: member.id.clone(),
                candidate: athlete.id.clone(),
                canonical: canonical.clone(),
                observation_digest: observation_digest.clone(),
                evidence_digest: census_domain::model::serialized_digest(evidence)?,
            };
            encoding::admit(&witness, grant)?;
            result.witnesses.push(witness);
            result.supported.insert(member.id.clone());
            if member.join.qualifies(athlete, &member.person)
                && capture.owner_span.is_ok()
                && member.spans.is_ok()
            {
                retain(capture, member, athlete, evidence, &note, audit)?;
            }
        }
        if !witnessed {
            record_gap(
                result,
                canonical,
                note.source_row_key().to_string(),
                grant,
                audit,
            )?;
        }
    }
    Ok(())
}

fn record_gap(
    result: &mut PhysicalEvidence,
    canonical: &AthleteId,
    reference: String,
    grant: &mut AttachmentReplayGrantV1,
    audit: &mut RetainedAudit,
) -> StoreResult<()> {
    let needed = u64::try_from(result.gaps.len())
        .map_err(|_| StoreError::CounterOverflow)?
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    if needed > OBSERVATION_BUDGET.rows {
        return Err(StoreError::BudgetExceeded {
            what: "physical roster gap inventory rows",
            needed,
            available: OBSERVATION_BUDGET.rows,
        });
    }
    let gap = PhysicalGap {
        athlete: canonical.clone(),
        reference,
        reason: "missing_reviewed_source_authority",
    };
    encoding::admit(&gap, grant)?;
    result
        .gaps
        .try_reserve(1)
        .map_err(|_| invalid("physical roster gap allocation failed"))?;
    result.gaps.push(gap);
    audit.gap(canonical.clone(), Gap::ArchiveUnavailable)?;
    Ok(())
}

fn retain(
    capture: &material::Capture,
    member: &material::Member,
    athlete: &CanonicalAthlete,
    evidence: &census_domain::model::Evidence,
    note: &RosterMemberObservationV1,
    audit: &mut RetainedAudit,
) -> StoreResult<()> {
    audit.context.retain(&athlete.id, evidence)?;
    if let Some(assertion) = member.located.current_roster() {
        audit.context.retain_dated(
            &athlete.id,
            evidence,
            super::super::context::DatedRosterMembership {
                observed_on: capture.proposal.capture().observed_day().clone(),
                observed_at: note.fetched_at().to_string(),
                day_convention: super::super::context::ObservationDayConvention::UtcCaptureDay,
                assertion: assertion.clone(),
                capture_sha256: note.capture_sha256().to_string(),
            },
        )?;
    }
    Ok(())
}

type Rows<'a> = BTreeMap<String, Vec<(&'a material::Capture, &'a material::Member)>>;

fn index<'a>(
    captures: &'a [material::Capture],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Rows<'a>> {
    let mut rows = BTreeMap::<String, Vec<(&material::Capture, &material::Member)>>::new();
    for capture in captures {
        for member in &capture.members {
            let row = member.located.locator();
            let key = format!(
                "{}#capture={}&row={}&offset={}&length={}",
                capture.proposal.capture().requested_url(),
                capture.proposal.capture().body().digest().as_str(),
                row.ordinal,
                row.byte_offset,
                row.byte_length
            );
            charge(grant, key.len(), 1)?;
            if let Some(previous) = rows.get(&key) {
                if previous.iter().any(|(previous_capture, previous_member)| {
                    previous_member.located != member.located
                        || previous_capture.proposal.scope().owner()
                            != capture.proposal.scope().owner()
                }) {
                    return Err(invalid(
                        "one captured row has conflicting reviewed interpretations",
                    ));
                }
            }
            rows.entry(key).or_default().push((capture, member));
        }
    }
    Ok(rows)
}

fn matching(
    capture: &material::Capture,
    member: &material::Member,
    athlete: &CanonicalAthlete,
    note: &RosterMemberObservationV1,
    ownership: PersonOwnership,
) -> StoreResult<()> {
    let source = member.located.athlete();
    let row = note.row();
    let end = row
        .byte_offset()
        .checked_add(row.byte_length())
        .ok_or(StoreError::CounterOverflow)?;
    let bytes = capture
        .body
        .as_bytes()
        .get(row.byte_offset()..end)
        .ok_or_else(|| invalid("physical roster row is outside capture"))?;
    person(capture, member, athlete, ownership)?;
    if hash(bytes) != note.row_sha256()
        || source.name != athlete.canonical_name
        || source.gender != athlete.gender
        || source.grad_year != athlete.grad_year
        || member.person.provider_id() != note.provider_athlete_id()
        || capture.proposal.scope().owner().provider_id() != note.provider_team_id()
        || capture.proposal.capture().requested_url() != note.capture_url()
        || capture.proposal.capture().observed_final_url() != note.response_url()
        || capture.proposal.capture().body().bytes().get()
            != u64::try_from(note.capture_bytes()).map_err(|_| StoreError::CounterOverflow)?
        || checked(chrono::DateTime::parse_from_rfc3339(
            capture.proposal.capture().acquired_at(),
        ))?
        .to_utc()
            != note
                .acquired_at()
                .map_err(|_| invalid("retained roster acquisition instant is invalid"))?
    {
        return Err(invalid("physical roster row differs from reviewed source"));
    }
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct PhysicalWitness {
    member: String,
    candidate: AthleteId,
    canonical: AthleteId,
    observation_digest: String,
    evidence_digest: String,
}

fn person(
    capture: &material::Capture,
    member: &material::Member,
    athlete: &CanonicalAthlete,
    ownership: PersonOwnership,
) -> StoreResult<()> {
    let source = member.located.athlete();
    let primary = athlete.source.as_ref().ok_or_else(|| {
        invalid("physical roster row differs from its source-owned school candidate")
    })?;
    let expected = CanonicalAthlete::mint(
        &capture.school.id,
        &source.name,
        source.grad_year,
        source.gender,
        primary,
    );
    let owns = primary.namespace == *member.person.namespace()
        && primary.id == member.person.provider_id();
    let linked = !owns
        && matches!(ownership, PersonOwnership::AcceptedSecondary)
        && athlete.source_links.iter().any(|link| {
            link.namespace == *member.person.namespace() && link.id == member.person.provider_id()
        });
    if athlete.school != capture.school.id || athlete.id != expected || !(owns || linked) {
        return Err(invalid(
            "physical roster row differs from its source-owned school candidate",
        ));
    }
    Ok(())
}

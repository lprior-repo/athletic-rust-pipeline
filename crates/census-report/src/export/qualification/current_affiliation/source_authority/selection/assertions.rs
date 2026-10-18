use super::*;
use census_domain::affiliation_candidates::OwnerSourceSpan;

pub(super) struct Inputs<'a> {
    pub(super) facts: Vec<LocallyCheckedAssertion<'a>>,
    pub(super) gapped: bool,
}

pub(super) fn assertions<'a>(
    athlete: &AthleteId,
    sport: Sport,
    members: &[(usize, usize)],
    supported: &BTreeSet<String>,
    captures: &'a [material::Capture],
    evidence: &'a [LocallyCheckedEvidence<'a>],
    published: &'a [Vec<&'a SourceSubject>],
    known: KnownBounds<'a>,
    audit: &mut RetainedAudit,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Inputs<'a>> {
    let mut facts = Vec::new();
    let mut gapped = false;
    for (capture_index, member_index) in members {
        let capture = captures
            .get(*capture_index)
            .ok_or_else(|| invalid("roster family capture index absent"))?;
        let evidence = evidence
            .get(*capture_index)
            .ok_or_else(|| invalid("roster family evidence index absent"))?;
        let published = published
            .get(*capture_index)
            .ok_or_else(|| invalid("roster family owner index absent"))?;
        let member = capture
            .members
            .get(*member_index)
            .ok_or_else(|| invalid("roster family member index absent"))?;
        charge(grant, 0, 1)?;
        if !applicable(capture, member, athlete, sport) {
            continue;
        }
        audit.reference(athlete, &member.id)?;
        match witness(capture, member, supported, athlete, audit)? {
            Some((owner, spans)) => {
                if facts.len() >= 256 {
                    return Err(invalid("current affiliation assertion ceiling exceeded"));
                }
                facts.push(super::fact::assertion(
                    capture, member, evidence, known, published, owner, spans,
                )?);
            }
            None => gapped = true,
        }
    }
    Ok(Inputs { facts, gapped })
}

fn applicable(
    capture: &material::Capture,
    member: &material::Member,
    athlete: &AthleteId,
    sport: Sport,
) -> bool {
    member.join.connected(athlete)
        && capture.proposal.scope().sport() == sport
        && member.located.athlete().gender == capture.proposal.scope().category()
        && !member
            .located
            .current_roster()
            .is_some_and(|assertion| !assertion.supports(sport))
}

fn witness<'a>(
    capture: &material::Capture,
    member: &'a material::Member,
    supported: &BTreeSet<String>,
    athlete: &AthleteId,
    audit: &mut RetainedAudit,
) -> StoreResult<
    Option<(
        OwnerSourceSpan,
        &'a census_crawl::milesplit::RosterRowStatementsV1,
    )>,
> {
    if !supported.contains(&member.id) {
        audit.gap(athlete.clone(), Gap::IdentityUnaccepted)?;
        audit.reference(
            athlete,
            &format!(
                "source_authority_gap:{}:missing_exact_physical_person_row",
                member.id
            ),
        )?;
        return Ok(None);
    }
    match (&capture.owner_span, &member.spans) {
        (Ok(owner), Ok(spans)) => Ok(Some((
            *owner
                .first()
                .ok_or_else(|| invalid("empty published owner span inventory"))?,
            spans,
        ))),
        _ => {
            audit.gap(athlete.clone(), Gap::MissingTemporalAuthority)?;
            audit.reference(
                athlete,
                &format!(
                    "source_authority_gap:{}:{:?}:{:?}",
                    member.id, capture.owner_span, member.spans
                ),
            )?;
            Ok(None)
        }
    }
}

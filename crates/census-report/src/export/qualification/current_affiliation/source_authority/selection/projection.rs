use super::*;

pub(super) fn retain(
    athlete: &AthleteId,
    sport: Sport,
    facts: &[FactDisposition<'_>],
    audit: &mut RetainedAudit,
) -> StoreResult<()> {
    for fact in facts {
        if let FactDisposition::Assertion { fact, disposition } = fact {
            audit.reference(
                athlete,
                &format!(
                    "roster_selector:{sport:?}:{}:{disposition:?}",
                    fact.raw().id.as_str()
                ),
            )?;
        }
    }
    Ok(())
}

pub(super) fn merge(
    resolution: &mut Resolution,
    outcome: CurrentAffiliationOutcome<'_>,
    facts: &[FactDisposition<'_>],
) {
    let incoming = match outcome {
        CurrentAffiliationOutcome::ResolvedSchoolCandidate(school) => Resolution::Resolved {
            school: school.clone(),
            evidence_refs: facts
                .iter()
                .filter_map(|fact| match fact {
                    FactDisposition::Assertion {
                        fact,
                        disposition: Disposition::Candidate,
                    } => Some(fact.raw().id.as_str().to_string()),
                    _ => None,
                })
                .collect(),
        },
        CurrentAffiliationOutcome::Conflicted => Resolution::Conflicted,
        CurrentAffiliationOutcome::CoopCampusUnresolved => Resolution::CoopCampusUnresolved,
        CurrentAffiliationOutcome::Unknown
        | CurrentAffiliationOutcome::SubjectUnresolved
        | CurrentAffiliationOutcome::EndedWithoutReplacement
        | CurrentAffiliationOutcome::NonSchool(_) => Resolution::Unknown,
    };
    match (&mut *resolution, incoming) {
        (Resolution::Unknown, next) => *resolution = next,
        (_, Resolution::Unknown) => {}
        (
            Resolution::Resolved {
                school,
                evidence_refs,
            },
            Resolution::Resolved {
                school: other,
                evidence_refs: more,
            },
        ) if *school == other => {
            evidence_refs.extend(more);
            evidence_refs.sort();
            evidence_refs.dedup();
        }
        (Resolution::CoopCampusUnresolved, Resolution::CoopCampusUnresolved) => {}
        _ => *resolution = Resolution::Conflicted,
    }
}

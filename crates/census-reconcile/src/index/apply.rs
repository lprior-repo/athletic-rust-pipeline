
use census_domain::model::{
    AppliedAthleteIdentity, AppliedIdentityKind, AthleteCandidateId, AthleteIdentityIndex,
    IdentityMember, IdentityProjectionBuilder, ReviewCase, ReviewState, ReviewVerdictRecord,
    SourceObjectIdentity,
};

use census_report::report::{ReportError, ReportResult};
use census_store::{Store, Table};

pub(super) fn apply_decisions(
    store: &Store,
    _identities: &[SourceObjectIdentity],
) -> ReportResult<usize> {
    let cases: Vec<ReviewCase> = store.scan(Table::ReviewCases)?;
    let verdicts: Vec<ReviewVerdictRecord> = store.scan(Table::IdentityVerdicts)?;

    let mut athletes: Vec<census_domain::model::CanonicalAthlete> = Vec::new();
    store.for_each_merged::<census_domain::model::CanonicalAthlete>(
        Table::Athletes,
        |a| {
            athletes.push(a);
            Ok(())
        },
    )?;

    let mut source_bound_index = AthleteIdentityIndex::default();
    for athlete in &athletes {
        if let Err(_e) = source_bound_index.observe(athlete) {
        }
    }

    let builder_index = build_index(&athletes);
    let mut builder = IdentityProjectionBuilder::new(builder_index, &cases, &verdicts)
        .map_err(|e| ReportError::Invariant {
            detail: format!("identity projection builder: {e}"),
        })?;

    let existing: Vec<AppliedAthleteIdentity> =
        store.scan(Table::AthleteIdentityDecisions)?;

    let mut decisions: Vec<AppliedAthleteIdentity> = Vec::new();
    for decision in &existing {
        match builder.consider(decision) {
            Ok(None) => {
                decisions.push(decision.clone());
            }
            Ok(Some(_)) => {
            }
            Err(_e) => {
            }
        }
    }

    for subject in source_bound_index.subjects() {
        if source_bound_index.isolated_source(subject.as_str()) {
            if let Some(fact) = source_bound_index.member(subject.as_str()) {
                let members = vec![IdentityMember {
                    subject: subject.clone(),
                    evidence_digest: fact.evidence_digest.clone(),
                }];
                decisions.push(AppliedAthleteIdentity {
                    id: format!("source_bound:{}:p1", subject),
                    policy: census_domain::model::ATHLETE_IDENTITY_POLICY,
                    kind: AppliedIdentityKind::SourceBound,
                    members,
                    canonical_id: None,
                    case_id: None,
                    verdict_digest: None,
                    observed_at: "derive".to_string(),
                });
            }
        }
    }

    store.replace_many(Table::AthleteIdentityDecisions, &decisions)?;

    Ok(decisions.len())
}

fn build_index(athletes: &[census_domain::model::CanonicalAthlete]) -> AthleteIdentityIndex {
    let mut index = AthleteIdentityIndex::default();
    for athlete in athletes {
        if let Err(_e) = index.observe(athlete) {
        }
    }
    index
}

pub(super) fn invalidate_stale_applications(store: &Store) -> ReportResult<usize> {
    let decisions: Vec<AppliedAthleteIdentity> = store.scan(Table::AthleteIdentityDecisions)?;
    if decisions.is_empty() {
        return Ok(0);
    }

    let mut athletes: Vec<census_domain::model::CanonicalAthlete> = Vec::new();
    store.for_each_merged::<census_domain::model::CanonicalAthlete>(
        Table::Athletes,
        |a| {
            athletes.push(a);
            Ok(())
        },
    )?;

    let index = build_index(&athletes);

    let verdicts: Vec<ReviewVerdictRecord> = store.scan(Table::IdentityVerdicts)?;
    let cases: Vec<ReviewCase> = store.scan(Table::ReviewCases)?;

    let mut invalidated = 0usize;

    for decision in &decisions {
        let mut stale = false;
        for member in &decision.members {
            let current_member = index.member(member.subject.as_str());
            match current_member {
                Some(current) => {
                    if current.evidence_digest != member.evidence_digest {
                        stale = true;
                        break;
                    }
                }
                None => {
                    stale = true;
                    break;
                }
            }
        }

        if !stale {
            if let Some(case_id) = &decision.case_id {
                if let Some(case) = cases.iter().find(|c| c.id == *case_id) {
                    match case.state {
                        ReviewState::Superseded | ReviewState::Pending => {
                            stale = true;
                        }
                        ReviewState::Resolved | ReviewState::Retained => {
                            let has_accepted_verdict = verdicts.iter().any(|v| {
                                v.case_id == *case_id
                                    && v.field == "identity"
                                    && (decision.kind == AppliedIdentityKind::SamePerson
                                        || decision.kind == AppliedIdentityKind::DifferentPerson)
                                    && v.accepted
                            });
                            if !has_accepted_verdict
                                && (decision.kind == AppliedIdentityKind::SamePerson
                                    || decision.kind == AppliedIdentityKind::DifferentPerson)
                            {
                                stale = true;
                            }
                        }
                    }
                } else {
                    stale = true;
                }
            }
        }

        if stale {
            invalidated += 1;
        }
    }

    Ok(invalidated)
}

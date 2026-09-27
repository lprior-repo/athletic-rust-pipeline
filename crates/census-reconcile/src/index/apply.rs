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
    let athletes = load_athletes(store)?;
    let source_bound_index = build_index(&athletes);
    let existing: Vec<AppliedAthleteIdentity> = store.scan(Table::AthleteIdentityDecisions)?;
    let mut decisions = retained_decisions(&athletes, &cases, &verdicts, &existing)?;
    decisions.extend(source_bound_decisions(&source_bound_index));
    store.replace_many(Table::AthleteIdentityDecisions, &decisions)?;
    Ok(decisions.len())
}

fn load_athletes(store: &Store) -> ReportResult<Vec<census_domain::model::CanonicalAthlete>> {
    let mut athletes: Vec<census_domain::model::CanonicalAthlete> = Vec::new();
    store.for_each_merged::<census_domain::model::CanonicalAthlete>(
        Table::Athletes,
        |athlete| {
            athletes.push(athlete);
            Ok(())
        },
    )?;
    Ok(athletes)
}

fn retained_decisions(
    athletes: &[census_domain::model::CanonicalAthlete],
    cases: &[ReviewCase],
    verdicts: &[ReviewVerdictRecord],
    existing: &[AppliedAthleteIdentity],
) -> ReportResult<Vec<AppliedAthleteIdentity>> {
    let builder_index = build_index(athletes);
    let mut builder =
        IdentityProjectionBuilder::new(builder_index, cases, verdicts).map_err(|error| {
            ReportError::Invariant {
                detail: format!("identity projection builder: {error}"),
            }
        })?;
    let mut decisions: Vec<AppliedAthleteIdentity> = Vec::new();
    for decision in existing {
        if let Ok(None) = builder.consider(decision) {
            decisions.push(decision.clone());
        }
    }
    Ok(decisions)
}

fn source_bound_decisions(index: &AthleteIdentityIndex) -> Vec<AppliedAthleteIdentity> {
    let mut decisions: Vec<AppliedAthleteIdentity> = Vec::new();
    for subject in index.subjects() {
        if !index.isolated_source(subject.as_str()) {
            continue;
        }
        let Some(fact) = index.member(subject.as_str()) else {
            continue;
        };
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
    decisions
}

fn build_index(athletes: &[census_domain::model::CanonicalAthlete]) -> AthleteIdentityIndex {
    let mut index = AthleteIdentityIndex::default();
    for athlete in athletes {
        if let Err(_error) = index.observe(athlete) {}
    }
    index
}

pub(super) fn invalidate_stale_applications(store: &Store) -> ReportResult<usize> {
    let decisions: Vec<AppliedAthleteIdentity> = store.scan(Table::AthleteIdentityDecisions)?;
    if decisions.is_empty() {
        return Ok(0);
    }
    let athletes = load_athletes(store)?;
    let index = build_index(&athletes);
    let verdicts: Vec<ReviewVerdictRecord> = store.scan(Table::IdentityVerdicts)?;
    let cases: Vec<ReviewCase> = store.scan(Table::ReviewCases)?;
    Ok(decisions
        .iter()
        .filter(|decision| decision_is_stale(decision, &index, &cases, &verdicts))
        .count())
}

fn decision_is_stale(
    decision: &AppliedAthleteIdentity,
    index: &AthleteIdentityIndex,
    cases: &[ReviewCase],
    verdicts: &[ReviewVerdictRecord],
) -> bool {
    members_diverged(decision, index) || case_is_stale(decision, cases, verdicts)
}

fn members_diverged(decision: &AppliedAthleteIdentity, index: &AthleteIdentityIndex) -> bool {
    decision.members.iter().any(|member| {
        index
            .member(member.subject.as_str())
            .is_none_or(|current| current.evidence_digest != member.evidence_digest)
    })
}

fn case_is_stale(
    decision: &AppliedAthleteIdentity,
    cases: &[ReviewCase],
    verdicts: &[ReviewVerdictRecord],
) -> bool {
    let Some(case_id) = decision.case_id.as_deref() else {
        return false;
    };
    let Some(case) = cases.iter().find(|case| case.id == case_id) else {
        return true;
    };
    match case.state {
        ReviewState::Superseded | ReviewState::Pending => true,
        ReviewState::Resolved | ReviewState::Retained => {
            needs_accepted_verdict(decision, case_id, verdicts)
        }
    }
}

fn needs_accepted_verdict(
    decision: &AppliedAthleteIdentity,
    case_id: &str,
    verdicts: &[ReviewVerdictRecord],
) -> bool {
    if !is_identity_pair(decision.kind) {
        return false;
    }
    !verdicts.iter().any(|verdict| {
        verdict.case_id == case_id && verdict.field == "identity" && verdict.accepted
    })
}

fn is_identity_pair(kind: AppliedIdentityKind) -> bool {
    matches!(
        kind,
        AppliedIdentityKind::SamePerson | AppliedIdentityKind::DifferentPerson
    )
}

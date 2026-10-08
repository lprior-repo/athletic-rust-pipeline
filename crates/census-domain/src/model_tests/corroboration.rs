use super::*;

#[path = "corroboration/captures.rs"]
mod captures;
#[path = "corroboration/cen14.rs"]
mod cen14;

use captures::{admits, captured_claim, independent_pair};

fn milesplit(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::MilesplitAthlete, id)
        .with_url(format!("https://milesplit.test/{id}"))
}

fn tfrrs(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::TfrrsAthlete, id)
        .with_url(format!("https://tfrrs.test/{id}"))
}

fn observed(school: &str, source: SourceIdentity, links: &[SourceIdentity]) -> CanonicalAthlete {
    let school = SchoolId::mint("sch", &["WI", school]);
    let mut athlete = CanonicalAthlete::new(
        &school,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        source,
    );
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new("fixture", Some(format!("https://fixture.test/{school}"))),
        "2026-10-07T12:00:00Z",
    ));
    for link in links {
        athlete.add_identity(link.clone());
    }
    athlete
}

#[test]
fn linked_object_corroborates_when_attested_independently() -> Result<(), Box<dyn std::error::Error>>
{
    check!(admits(&independent_pair()?)?);
    Ok(())
}

#[test]
fn a_link_shared_pair_without_attestation_stays_review() -> Result<(), Box<dyn std::error::Error>> {
    let mut rows = independent_pair()?;
    for row in &mut rows {
        row.identity_attestations.clear();
    }
    check!(!admits(&rows)?);
    rows.reverse();
    check!(!admits(&rows)?);
    Ok(())
}

#[test]
fn two_ids_from_one_provider_withhold() -> Result<(), Box<dyn std::error::Error>> {
    let [first, owner] = independent_pair()?;
    let mut second = observed("School C", milesplit("222"), &[tfrrs("999")]);
    second.identity_attestations.push(captured_claim(
        &tfrrs("999"),
        "other-roster-fixture",
        "other-school-fixture",
    )?);
    check!(!admits(&[first, second, owner])?);
    Ok(())
}

#[test]
fn an_unparsed_member_withholds_corroboration() -> Result<(), Box<dyn std::error::Error>> {
    let [linked, mut owner] = independent_pair()?;
    for evidence in &mut owner.evidence {
        evidence.method = EvidenceMethod::Fetched;
    }
    check!(!admits(&[linked, owner])?);
    Ok(())
}

#[test]
fn a_contradictory_third_member_withholds() -> Result<(), Box<dyn std::error::Error>> {
    let rows = independent_pair()?;
    check!(admits(&rows)?);
    let [linked, owner] = rows;
    let mut third = observed("School C", tfrrs("777"), &[]);
    third.gender = Gender::Boys;
    check!(!admits(&[linked, owner, third])?);
    Ok(())
}

#[test]
fn name_school_and_cohort_agreement_alone_is_not_positive_evidence(
) -> Result<(), Box<dyn std::error::Error>> {
    check!(!admits(&[
        observed("School A", milesplit("111"), &[]),
        observed("School A", milesplit("222"), &[]),
    ])?);
    Ok(())
}

fn resolved_identity_cases(
    index: &AthleteIdentityIndex,
    pairs: &[(&CanonicalAthlete, &CanonicalAthlete)],
) -> Result<(Vec<ReviewCase>, Vec<ReviewVerdictRecord>), Box<dyn std::error::Error>> {
    let mut cases = Vec::new();
    let mut verdicts = Vec::new();
    for (ordinal, (first, second)) in pairs.iter().enumerate() {
        let ids = vec![first.id.cast(), second.id.cast()];
        let subject = "Union component identity";
        let detail = format!("Shared provider document attests runner pair {ordinal}");
        let evidence = index.case_evidence(subject, &detail, &ids)?;
        let mut case = ReviewCase::pending_with_evidence(
            ATHLETE_IDENTITY_FAMILY,
            first.id.as_str(),
            subject,
            &detail,
            evidence,
        );
        case.member_ids = ids;
        case.state = ReviewState::Resolved;
        let verdict = ReviewVerdictRecord {
            id: case.id.clone(),
            case_id: case.id.clone(),
            subject_id: case.subject_id.clone(),
            family: case.family.clone(),
            kind: "value_proposed".into(),
            field: "identity".into(),
            value: "same_person".into(),
            accepted: true,
            confidence: 100,
            rationale: detail,
            reviewer: "deterministic fixture".into(),
            observed_at: "2026-10-02".into(),
            member_ids: case.member_ids.clone(),
        };
        cases.push(case);
        verdicts.push(verdict);
    }
    Ok((cases, verdicts))
}

fn accepted_component_decisions(
    athletes: &[&CanonicalAthlete],
    pairs: &[(usize, usize)],
) -> Result<
    (
        Vec<ReviewCase>,
        Vec<ReviewVerdictRecord>,
        Vec<AppliedAthleteIdentity>,
    ),
    Box<dyn std::error::Error>,
> {
    let mut index = AthleteIdentityIndex::default();
    for athlete in athletes {
        index.observe(athlete)?;
    }
    let pairs = pairs
        .iter()
        .map(|(left, right)| (athletes[*left], athletes[*right]))
        .collect::<Vec<_>>();
    let (cases, verdicts) = resolved_identity_cases(&index, &pairs)?;
    let builder = IdentityProjectionBuilder::new(index, &cases, &verdicts)?;
    let decisions = builder
        .reviewed_applications("2026-10-02")
        .map(|(_, result)| match result? {
            IdentityApplication::Accepted(decision) => Ok(decision.record().clone()),
            IdentityApplication::Retained(issue) => {
                Err(format!("fixture identity rejected: {issue:?}").into())
            }
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    Ok((cases, verdicts, decisions))
}

fn component_projection(
    athletes: &[&CanonicalAthlete],
    cases: &[ReviewCase],
    verdicts: &[ReviewVerdictRecord],
    decisions: &[&AppliedAthleteIdentity],
) -> Result<AthleteIdentityProjection, Box<dyn std::error::Error>> {
    let mut index = AthleteIdentityIndex::default();
    for athlete in athletes {
        index.observe(athlete)?;
    }
    let mut builder = IdentityProjectionBuilder::new(index, cases, verdicts)?;
    for decision in decisions {
        check!(eq; builder.consider(decision)?, None);
    }
    Ok(builder.finish()?)
}

#[test]
fn union_component_provider_conflict_withholds_aliases_in_either_application_order(
) -> Result<(), Box<dyn std::error::Error>> {
    let [first, mut owner] = independent_pair()?;
    owner.add_identity(milesplit("111").with_url("https://tfrrs.test/999"));
    let key = tfrrs("999");
    let mut second = observed("School C", milesplit("222"), std::slice::from_ref(&key));
    second.identity_attestations.push(captured_claim(
        &key,
        "other-roster-fixture",
        "other-school-fixture",
    )?);
    let athletes = [&first, &owner, &second];
    let (cases, verdicts, decisions) = accepted_component_decisions(&athletes, &[(0, 1), (1, 2)])?;
    let forward = component_projection(
        &athletes,
        &cases,
        &verdicts,
        &[&decisions[0], &decisions[1]],
    )?;
    let reverse = component_projection(
        &athletes,
        &cases,
        &verdicts,
        &[&decisions[1], &decisions[0]],
    )?;
    for athlete in athletes {
        let subject = athlete.id.as_str();
        check!(eq; forward.status(subject)?, IdentityStatus::RetainedConflict);
        check!(eq; reverse.status(subject)?, IdentityStatus::RetainedConflict);
        check!(eq; forward.canonical_id(subject), subject);
        check!(eq; reverse.canonical_id(subject), subject);
    }
    check!(eq; forward
        .rejected_applications()
        .get(&IdentityDecisionIssue::ConflictingApplications),
    Some(&1),);
    check!(eq; reverse
        .rejected_applications()
        .get(&IdentityDecisionIssue::ConflictingApplications),
    Some(&1),);
    check!(eq; forward.rejected_applications().len(), 1);
    check!(eq; reverse.rejected_applications().len(), 1);
    Ok(())
}

#[test]
fn compatible_provider_chain_component_publishes_one_verified_person(
) -> Result<(), Box<dyn std::error::Error>> {
    let [first, mut owner] = independent_pair()?;
    owner.add_identity(milesplit("111").with_url("https://tfrrs.test/999"));
    let key = tfrrs("999");
    let mut mirror = observed("School D", milesplit("111"), std::slice::from_ref(&key));
    mirror.identity_attestations.push(captured_claim(
        &key,
        "compatible-roster-fixture",
        "compatible-school-fixture",
    )?);
    let athletes = [&first, &owner, &mirror];
    let (cases, verdicts, decisions) = accepted_component_decisions(&athletes, &[(0, 1), (1, 2)])?;
    let projection = component_projection(
        &athletes,
        &cases,
        &verdicts,
        &[&decisions[0], &decisions[1]],
    )?;
    let canonical = projection.canonical_id(first.id.as_str()).to_owned();
    for athlete in athletes {
        check!(eq; projection.status(athlete.id.as_str())?,
        IdentityStatus::Verified);
        check!(eq; projection.canonical_id(athlete.id.as_str()), canonical.as_str());
    }
    check!(eq; projection.rejected_applications().len(), 0);
    Ok(())
}

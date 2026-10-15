use super::super::TestResult;
use super::fixture::DAY;
use census_domain::model::*;
use census_store::{Store, Table};

pub(super) fn members(school: &SchoolId) -> TestResult<[CanonicalAthlete; 2]> {
    let member = |name: &str| -> TestResult<CanonicalAthlete> {
        let source = SourceRef::new(
            "fixture_roster",
            Some("https://roster.example.test/9001001".into()),
        );
        let mut athlete = CanonicalAthlete::new(
            school,
            name,
            GradYear::CO2027,
            Gender::Girls,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9001001")
                .with_url("https://mi.milesplit.com/athletes/9001001"),
        )?;
        athlete.published_graduations.push(PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: source.clone(),
        });
        athlete.sports.push(Sport::OutdoorTrack);
        athlete.evidence.push(Evidence::parsed(source, DAY));
        Ok(athlete)
    };
    let members = [member("Merge Runner")?, member("M. Runner")?];
    check!(members[0].id != members[1].id);
    Ok(members)
}

pub(super) fn accept(store: &Store, members: &[CanonicalAthlete; 2]) -> TestResult<String> {
    let index = store.athlete_identity_index()?;
    let ids = members
        .iter()
        .map(|athlete| athlete.id.cast())
        .collect::<Vec<_>>();
    let subject = "Source-backed normalized readback fixture";
    let detail = "One published provider identifier retained with alternate names";
    let evidence = index.case_evidence(subject, detail, &ids)?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        members[0].id.as_str(),
        subject,
        detail,
        evidence,
    );
    case.member_ids = ids;
    case.state = ReviewState::Resolved;
    let verdict = verdict(&case, detail);
    store.replace(Table::ReviewCases, &case)?;
    store.replace(Table::IdentityVerdicts, &verdict)?;
    let cases = [case];
    let verdicts = [verdict];
    let builder = IdentityProjectionBuilder::new(index, &cases, &verdicts)?;
    let decisions = builder
        .reviewed_applications(DAY)
        .map(|(_, result)| -> TestResult<_> {
            match result? {
                IdentityApplication::Accepted(decision) => Ok(decision),
                IdentityApplication::Retained(issue) => {
                    Err(format!("positive fixture identity refused: {issue:?}").into())
                }
            }
        })
        .collect::<TestResult<Vec<_>>>()?;
    check!(eq; decisions.len(), 1);
    let decision = decisions
        .first()
        .ok_or("missing identity decision")?
        .record()
        .id
        .clone();
    check!(eq; store.apply_identity_decisions(&decisions)?, 1);
    Ok(decision)
}

fn verdict(case: &ReviewCase, detail: &str) -> ReviewVerdictRecord {
    ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        kind: "value_proposed".into(),
        field: "identity".into(),
        value: "same_person".into(),
        accepted: true,
        confidence: 100,
        rationale: detail.into(),
        reviewer: "deterministic fixture".into(),
        observed_at: DAY.into(),
        member_ids: case.member_ids.clone(),
    }
}

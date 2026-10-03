use crate::{Store, Table};
use census_domain::model::{
    athlete_identity_digest, AppliedAthleteIdentity, AppliedIdentityKind, CanonicalAthlete, Gender,
    GradYear, Grade, IdentityDecisionIssue, IdentityMember, IdentityStatus, ObservedGrade,
    ReviewCase, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
    ATHLETE_IDENTITY_FAMILY, ATHLETE_IDENTITY_POLICY,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn subject(school: &SchoolId, native_id: &str) -> TestResult<CanonicalAthlete> {
    let mut athlete = CanonicalAthlete::new(
        school,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, native_id),
    );
    athlete.observed_grades.push(ObservedGrade {
        grade: Grade::new(11).ok_or("valid fixture grade")?,
        school_year: SchoolYear::new(2025).ok_or("valid fixture year")?,
        source: SourceRef::id("synthetic:published-grade"),
    });
    Ok(athlete)
}

fn assert_projection(
    store: &Store,
    first: &CanonicalAthlete,
    second: &CanonicalAthlete,
) -> TestResult {
    let projection = store.athlete_identity_projection()?;
    {
        let (left, right) = (
            &projection.status(first.id.as_str())?,
            &IdentityStatus::Unverified,
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &projection.status(second.id.as_str())?,
            &IdentityStatus::Pending,
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &projection.canonical_id(first.id.as_str()),
            &first.id.as_str(),
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &projection.canonical_id(second.id.as_str()),
            &second.id.as_str(),
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &projection
                .rejected_applications()
                .get(&IdentityDecisionIssue::CompetingSourceClaims),
            &Some(&1),
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    let retained: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    {
        let (left, right) = (&retained.len(), &2);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    if !retained.contains(first) {
        return Err(format!("missing first athlete {first:?} in {retained:?}").into());
    }
    if !retained.contains(second) {
        return Err(format!("missing second athlete {second:?} in {retained:?}").into());
    }
    Ok(())
}

#[test]
fn unsupported_application_cannot_verify_homonyms_or_block_projection_after_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let school = SchoolId::mint("sch", &["identity-projection-fixture"]);
    let first = subject(&school, "1001")?;
    let second = subject(&school, "1002")?;
    let application = AppliedAthleteIdentity {
        id: "unsupported-source-binding".to_owned(),
        policy: ATHLETE_IDENTITY_POLICY,
        kind: AppliedIdentityKind::SourceBound,
        members: vec![IdentityMember {
            subject: first.id.cast(),
            evidence_digest: athlete_identity_digest(&first)?,
        }],
        canonical_id: Some(first.id.clone()),
        case_id: None,
        verdict_digest: None,
        observed_at: "2026-08-26T00:00:00Z".to_owned(),
    };
    {
        let store = Store::open(directory.path())?;
        store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
        store.replace(Table::AthleteIdentityDecisions, &application)?;
        store.replace(
            Table::ReviewCases,
            &ReviewCase::pending(
                ATHLETE_IDENTITY_FAMILY,
                second.id.as_str(),
                "Synthetic Runner",
                "Unresolved identity context",
            ),
        )?;
        assert_projection(&store, &first, &second)?;
    }
    assert_projection(&Store::open(directory.path())?, &first, &second)
}

use crate::model::{
    AppliedAthleteIdentity, AppliedIdentityKind, AthleteIdentityIndex, CanonicalAthlete, Evidence,
    Gender, GradYear, IdentityDecisionIssue, IdentityProjectionBuilder, IdentityStatus,
    PublishedGraduation, SchoolId, SourceIdentity, SourceNamespace, SourceRef,
    ATHLETE_IDENTITY_POLICY,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn athlete() -> CanonicalAthlete {
    let mut row = CanonicalAthlete::new(
        &SchoolId::mint("sch", &["published-identity-fixture"]),
        "Published Cohort Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001"),
    );
    row.published_graduations.push(published("owned-api"));
    row.evidence.push(Evidence::parsed(
        SourceRef::new(
            "owned-api",
            Some("https://source.example/athletes/1001".to_string()),
        ),
        "2026-10-02",
    ));
    row
}

fn published(source: &str) -> PublishedGraduation {
    PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(source, Some(format!("https://source.example/{source}"))),
    }
}

fn current_application(
    row: &CanonicalAthlete,
) -> Result<(AppliedAthleteIdentity, String), Box<dyn std::error::Error>> {
    let mut index = AthleteIdentityIndex::default();
    index.observe(row)?;
    let member = index
        .member(&row.id.cast())
        .ok_or("missing fixture subject")?;
    let evidence = index
        .case_evidence("Runner", "Identity review", &[row.id.cast()])?
        .digest();
    let application = AppliedAthleteIdentity {
        id: "published-cohort-identity".to_string(),
        policy: ATHLETE_IDENTITY_POLICY,
        kind: AppliedIdentityKind::SourceBound,
        members: vec![member],
        canonical_id: Some(row.id.clone()),
        case_id: None,
        verdict_digest: None,
        observed_at: "2026-10-02T00:00:00Z".to_string(),
    };
    let mut builder = IdentityProjectionBuilder::new(index, &[], &[])?;
    check!(eq; builder.consider(&application)?, None);
    check!(eq; builder.finish()?.status(row.id.as_str())?,
    IdentityStatus::Verified);
    Ok((application, evidence))
}

#[test]
fn changing_typed_published_claims_invalidates_identity_applications_and_case_evidence(
) -> TestResult {
    let original = athlete();
    let (application, evidence) = current_application(&original)?;
    let mut replaced_source = original.clone();
    replaced_source.published_graduations = vec![published("other-api")];
    let mut replaced_year = original.clone();
    replaced_year.published_graduations[0].grad_year =
        GradYear::new(2028).ok_or("invalid fixture year")?;
    let mut replaced_url = original.clone();
    replaced_url.published_graduations[0].source.url =
        Some("https://other.example/1001".to_string());
    let mut added = original.clone();
    added.published_graduations.push(published("other-api"));
    let mut removed = original;
    removed.published_graduations.clear();
    [replaced_source, replaced_year, replaced_url, added, removed]
        .into_iter()
        .try_for_each(|row| assert_stale_publication(&row, &application, &evidence))
}

fn assert_stale_publication(
    row: &CanonicalAthlete,
    application: &AppliedAthleteIdentity,
    evidence_digest: &str,
) -> TestResult {
    let mut index = AthleteIdentityIndex::default();
    index.observe(row)?;
    check!(ne; index
        .case_evidence("Runner", "Identity review", &[row.id.cast()])?
        .digest(),
    evidence_digest,);
    let mut builder = IdentityProjectionBuilder::new(index, &[], &[])?;
    check!(eq; builder.consider(application)?,
    Some(IdentityDecisionIssue::StaleMemberEvidence),);
    Ok(())
}

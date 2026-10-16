use super::*;
use census_domain::model::{ProfileDate, ProfileId};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod contacts;
mod fixture;
mod geography;
mod replay;

use fixture::Fixture;

fn view(
    fixture: &Fixture,
    as_of: &str,
    resolution: Option<CurrentAffiliationResolution>,
) -> TestResult<VerifiedCurrentAffiliations> {
    Ok(VerifiedCurrentAffiliations::fixture(
        fixture.dataset.lineage.input_generation.clone(),
        ProfileId::parse(&"a".repeat(64))?,
        ProfileDate::parse(as_of)?,
        resolution
            .into_iter()
            .map(|row| (fixture.athlete.clone(), row))
            .collect(),
    ))
}

fn resolved(school: &SchoolId, evidence: &str) -> CurrentAffiliationResolution {
    CurrentAffiliationResolution::Resolved {
        school: school.clone(),
        evidence_refs: vec![evidence.into()],
    }
}

fn bind_selected(fixture: &mut Fixture, affiliations: &VerifiedCurrentAffiliations) -> TestResult {
    let rule = census_domain::participation_candidates::participation_policy_v2()?;
    fixture
        .dataset
        .bind_participation_rule_for_test(rule.clone())?;
    fixture
        .dataset
        .bind_public_identity_fixture_from_store(&fixture.store)?;
    let affiliations = VerifiedCurrentAffiliations::fixture(
        fixture.dataset.lineage.input_generation.clone(),
        affiliations.profile_id().clone(),
        affiliations.as_of().clone(),
        [(
            fixture.athlete.clone(),
            affiliations.resolution(&fixture.athlete).clone(),
        )]
        .into_iter()
        .collect(),
    );
    let participation = crate::export::VerifiedParticipationView::fixture(
        fixture.dataset.lineage.input_generation.clone(),
        affiliations.profile_id().clone(),
        rule,
        BTreeMap::new(),
    );
    fixture
        .dataset
        .bind_verified_views_for_test(participation, affiliations)?;
    Ok(())
}

#[test]
fn no_current_school_change_preserves_borrowed_population() -> TestResult {
    let mut fixture = Fixture::at_cutoff("2026-09-30")?;
    let affiliations = view(
        &fixture,
        "2026-09-30",
        Some(resolved(&fixture.previous, "old-claim")),
    )?;
    bind_selected(&mut fixture, &affiliations)?;
    let derivation = Derivation::of(
        &fixture.dataset,
        Scope::AllSources,
        None,
        &[UsJurisdiction::Wisconsin],
    )?;
    check!(matches!(derivation.athletes, Cow::Borrowed(_)));
    check!(eq; derivation.athletes().as_ptr(), fixture.dataset.athletes.as_ptr());
    Ok(())
}

#[test]
fn missing_current_school_refuses_instead_of_falling_back() -> TestResult {
    let fixture = Fixture::new()?;
    let missing = SchoolId::mint("sch", &["missing current school"]);
    let affiliations = view(&fixture, "2026-10-01", Some(resolved(&missing, "claim")))?;
    check!(
        matches!(Derivation::selected(&fixture.dataset, Scope::AllSources, None, &[], &affiliations),
        Err(ReportError::Invariant { detail }) if detail == "resolved current school is absent from frozen input")
    );
    Ok(())
}

#[test]
fn foreign_input_generation_is_rejected() -> TestResult {
    let fixture = Fixture::new()?;
    let affiliations = VerifiedCurrentAffiliations::fixture(
        "foreign-generation".into(),
        ProfileId::parse(&"a".repeat(64))?,
        ProfileDate::parse("2026-10-01")?,
        [(fixture.athlete.clone(), resolved(&fixture.current, "claim"))]
            .into_iter()
            .collect(),
    );
    check!(
        matches!(Derivation::selected(&fixture.dataset, Scope::AllSources, None, &[], &affiliations),
        Err(ReportError::Invariant { detail }) if detail == "current affiliation belongs to a different frozen input")
    );
    Ok(())
}

#[test]
fn foreign_profile_and_cutoff_are_rejected() -> TestResult {
    let fixture = Fixture::new()?;
    for (profile, cutoff) in [
        ("b".repeat(64), "2026-10-01"),
        ("a".repeat(64), "2026-09-30"),
    ] {
        let affiliations = VerifiedCurrentAffiliations::fixture(
            fixture.dataset.lineage.input_generation.clone(),
            ProfileId::parse(&profile)?,
            ProfileDate::parse(cutoff)?,
            [(fixture.athlete.clone(), resolved(&fixture.current, "claim"))]
                .into_iter()
                .collect(),
        );
        check!(Derivation::selected(
            &fixture.dataset,
            Scope::AllSources,
            None,
            &[],
            &affiliations
        )
        .is_err());
    }
    Ok(())
}

#[test]
fn absent_selected_binding_is_not_inferred_from_resolution() -> TestResult {
    let fixture = Fixture::retained()?;
    let affiliations = view(
        &fixture,
        "2026-10-01",
        Some(resolved(&fixture.current, "claim")),
    )?;
    check!(Derivation::selected(
        &fixture.dataset,
        Scope::AllSources,
        None,
        &[],
        &affiliations
    )
    .is_err());
    Ok(())
}

#[test]
fn accepted_identity_without_participation_proof_still_refuses_public_derivation() -> TestResult {
    let fixture = Fixture::new()?;
    check!(fixture
        .dataset
        .verified_public_identities()?
        .public_id(fixture.athlete.as_str())
        .is_some());
    check!(
        matches!(Derivation::of(&fixture.dataset, Scope::AllSources, None, &[]),
        Err(ReportError::Invariant { detail })
        if detail == "selected publication requires retained participation verification")
    );
    Ok(())
}

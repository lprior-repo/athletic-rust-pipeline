use census_domain::model::{
    CanonicalCoach, CoachRole, CoachTenure, CoachTenureEvidence, Evidence, Gender, SchoolId,
    SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn coach() -> CanonicalCoach {
    CanonicalCoach::new(
        &SchoolId::mint("sch", &["synthetic"]),
        "Synthetic coach",
        Some(Sport::OutdoorTrack),
        Gender::Boys,
        CoachRole::HeadCoach,
    )
}

#[test]
fn source_and_date_are_selected_from_the_newest_url_bearing_observation() {
    let mut coach = coach();
    for (url, date) in [
        (Some("https://example.invalid/newest"), "2026-03-20"),
        (Some("https://example.invalid/older"), "2026-02-10"),
        (None, "2026-04-01"),
    ] {
        coach.evidence.push(Evidence::fetched(
            SourceRef::new("fixture", url.map(str::to_owned)),
            date,
        ));
    }
    assert_eq!(
        crate::export::coach_source(&coach),
        (Some("https://example.invalid/newest"), Some("2026-03-20"))
    );
    coach.evidence.reverse();
    assert_eq!(
        crate::export::coach_source(&coach),
        (Some("https://example.invalid/newest"), Some("2026-03-20"))
    );
}

#[test]
fn identity_url_fallback_has_no_unrelated_observation_date() {
    let mut coach = coach();
    coach.evidence.push(Evidence::fetched(
        SourceRef::new("fixture", None),
        "2026-04-01",
    ));
    coach.source_identities.push(
        SourceIdentity::new(SourceNamespace::Other("fixture".into()), "coach")
            .with_url("https://example.invalid/profile"),
    );
    assert_eq!(
        crate::export::coach_source(&coach),
        (Some("https://example.invalid/profile"), None)
    );
}

fn tenure_fact(url: Option<&str>, retrieved_at: &str) -> TestResult<CoachTenureEvidence> {
    Ok(CoachTenureEvidence {
        tenure: CoachTenure::Current {
            school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        },
        source: SourceRef::new("fixture", url.map(str::to_owned)),
        source_sha256: "c".repeat(64),
        retrieved_at: retrieved_at.to_string(),
        statement: String::from("fixture statement"),
        claim: None,
    })
}

#[test]
fn a_tenure_only_capture_is_the_coach_source() -> TestResult {
    let mut coach = coach();
    coach.evidence.push(Evidence::fetched(
        SourceRef::new("fixture", None),
        "2026-06-01",
    ));
    coach.tenure_evidence.push(tenure_fact(
        Some("https://example.invalid/tenure"),
        "2026-05-02",
    )?);
    assert_eq!(
        crate::export::coach_source(&coach),
        (Some("https://example.invalid/tenure"), Some("2026-05-02"))
    );
    Ok(())
}

#[test]
fn a_tenure_capture_outranks_generic_coach_evidence() -> TestResult {
    let mut coach = coach();
    coach.tenure_evidence.push(tenure_fact(
        Some("https://example.invalid/claim"),
        "2026-03-01",
    )?);
    coach.evidence.push(Evidence::fetched(
        SourceRef::new(
            "fixture",
            Some(String::from("https://example.invalid/newer")),
        ),
        "2026-04-01",
    ));
    assert_eq!(
        crate::export::coach_source(&coach),
        (Some("https://example.invalid/claim"), Some("2026-03-01"))
    );
    coach.tenure_evidence.push(tenure_fact(
        Some("https://example.invalid/claim-2"),
        "2026-05-01",
    )?);
    assert_eq!(
        crate::export::coach_source(&coach),
        (Some("https://example.invalid/claim-2"), Some("2026-05-01"))
    );
    Ok(())
}

#[test]
fn a_tenure_fact_without_a_url_falls_back_to_coach_evidence() -> TestResult {
    let mut coach = coach();
    coach.tenure_evidence.push(tenure_fact(None, "2026-05-02")?);
    coach.evidence.push(Evidence::fetched(
        SourceRef::new(
            "fixture",
            Some(String::from("https://example.invalid/observation")),
        ),
        "2026-04-01",
    ));
    assert_eq!(
        crate::export::coach_source(&coach),
        (
            Some("https://example.invalid/observation"),
            Some("2026-04-01")
        )
    );
    Ok(())
}

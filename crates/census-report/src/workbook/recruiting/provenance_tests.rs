use census_domain::model::{
    CanonicalCoach, CoachRole, Evidence, Gender, SchoolId, SourceIdentity, SourceNamespace,
    SourceRef, Sport,
};

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

use super::{AccessBlockKind, SourceAccessCondition};

fn condition() -> SourceAccessCondition {
    SourceAccessCondition::new(
        "source",
        "example.org",
        AccessBlockKind::RateLimited,
        429,
        "2026-10-18T10:00:00Z",
        "retained",
    )
}

#[test]
fn source_access_deadline_compares_instants_before_at_and_after_expiry() {
    let row = condition().with_cooldown_until(Some("2026-10-18T10:05:00Z".into()));
    assert!(row.is_blocking("2026-10-18T10:04:59Z"));
    assert!(!row.is_blocking("2026-10-18T10:05:00Z"));
    assert!(!row.is_blocking("2026-10-18T10:05:01Z"));
    assert!(!row.is_blocking("2026-10-18T12:05:00+02:00"));
}

#[test]
fn source_access_maximum_deadline_remains_future_until_its_exact_instant() {
    let maximum =
        chrono::DateTime::<chrono::Utc>::MAX_UTC.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let row = condition().with_cooldown_until(Some(maximum.clone()));
    assert!(row.is_blocking("2026-10-18T10:05:00Z"));
    assert!(!row.is_blocking(&maximum));
}

#[test]
fn source_access_indefinite_human_hold_and_invalid_instants_fail_closed() {
    let mut human = condition();
    human.kind = AccessBlockKind::HumanRequired;
    assert!(human.is_blocking("2099-01-01T00:00:00Z"));
    assert!(condition()
        .with_cooldown_until(Some("malformed".into()))
        .is_blocking("2026-10-18T10:05:00Z"));
    assert!(condition()
        .with_cooldown_until(Some("2020-01-01T00:00:00Z".into()))
        .is_blocking("malformed"));
}

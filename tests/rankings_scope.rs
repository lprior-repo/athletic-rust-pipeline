//! Scope contract: one rankings scope is one seasonal division, and only the
//! evidence-backed division lists may enter it.

use athletic_rust_pipeline::runtime::rankings::{
    expected_revision, season_list_id, RankingsScope, SeasonKind, SEASON_YEAR,
};

fn scope(season_kind: SeasonKind, gender: &str) -> RankingsScope {
    RankingsScope::for_division(season_kind, gender, 50).expect("supported division")
}

#[test]
fn canonical_requested_scope_is_the_outdoor_boys_division() {
    let requested = RankingsScope::requested(50).expect("canonical scope");
    let explicit = scope(SeasonKind::Outdoor, "m");
    assert!(requested.validate().is_ok());
    assert_eq!(requested.list_id, explicit.list_id);
    assert_eq!(requested.revision, explicit.revision);
    assert_eq!(requested.season_kind, SeasonKind::Outdoor);
    assert_eq!(requested.gender, "m");
    assert_eq!(requested.season, SEASON_YEAR);
}

#[test]
fn every_supported_division_binds_its_own_list_and_revision() {
    let mut seen = Vec::new();
    for season_kind in [SeasonKind::Outdoor, SeasonKind::Indoor] {
        for gender in ["m", "f"] {
            let scope = scope(season_kind, gender);
            scope.validate().expect("supported division validates");
            assert_eq!(
                scope.list_id,
                season_list_id(season_kind, SEASON_YEAR).expect("list")
            );
            assert_eq!(
                scope.revision,
                expected_revision(season_kind, gender).expect("revision")
            );
            assert_eq!(scope.season, SEASON_YEAR);
            seen.push((scope.list_id, scope.gender.clone(), scope.revision.clone()));
        }
    }
    // Indoor and outdoor are distinct divisions, and gender is part of the key.
    assert_eq!(seen.len(), 4);
    let mut keys: Vec<_> = seen
        .iter()
        .map(|(id, gender, _)| (*id, gender.clone()))
        .collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 4, "each division key must be distinct");
    let revisions: std::collections::BTreeSet<_> =
        seen.iter().map(|(_, _, revision)| revision).collect();
    assert_eq!(revisions.len(), 4, "each division must own a revision");
}

#[test]
fn indoor_scope_cannot_carry_the_outdoor_list_or_revision() {
    let mut tampered = scope(SeasonKind::Indoor, "m");
    tampered.list_id = 168_416;
    let error = tampered
        .validate()
        .expect_err("outdoor list under indoor kind");
    assert!(
        error
            .to_string()
            .contains("indoor 2026 division list must be 173005"),
        "unexpected error: {error}"
    );

    let mut tampered = scope(SeasonKind::Indoor, "m");
    tampered.revision = expected_revision(SeasonKind::Outdoor, "m").expect("revision");
    let error = tampered
        .validate()
        .expect_err("outdoor revision under indoor kind");
    assert!(
        error
            .to_string()
            .contains("unsupported ranking scope revision"),
        "unexpected error: {error}"
    );
}

#[test]
fn girls_scope_cannot_carry_the_boys_revision() {
    let mut tampered = scope(SeasonKind::Outdoor, "f");
    tampered.revision = "2026-usa-hs-grade11-outdoor-boys-v2".to_owned();
    let error = tampered
        .validate()
        .expect_err("boys revision under girls scope");
    assert!(
        error
            .to_string()
            .contains("unsupported ranking scope revision"),
        "unexpected error: {error}"
    );
}

#[test]
fn unsupported_gender_is_rejected_before_any_scope_exists() {
    let error =
        RankingsScope::for_division(SeasonKind::Outdoor, "x", 50).expect_err("unknown gender code");
    assert!(
        error.to_string().contains("unsupported gender"),
        "unexpected error: {error}"
    );
}

#[test]
fn deserialized_scope_must_revalidate_its_division() {
    let mut raw = serde_json::to_value(scope(SeasonKind::Indoor, "f")).expect("serialize");
    let scope: RankingsScope = serde_json::from_value(raw.clone()).expect("deserialize");
    scope.validate().expect("persisted scope validates");

    // A persisted indoor scope re-pointed at the outdoor list is not a
    // supported division even though every field is still well-formed.
    raw["list_id"] = serde_json::json!(168_416u64);
    let tampered: RankingsScope = serde_json::from_value(raw).expect("deserialize");
    assert!(tampered.validate().is_err());
}

use super::{resolve_restriction, resolve_states};
use census_domain::UsJurisdiction;

#[test]
fn all_states_selects_the_census_run_scope() {
    let states = resolve_states(true, &[]).expect("--all-states resolves");
    assert_eq!(states, UsJurisdiction::CENSUS_SCOPE);
    assert!(!states.contains(&UsJurisdiction::Alaska));
}

#[test]
fn no_flag_defaults_to_wisconsin() {
    let states = resolve_states(false, &[]).expect("the default resolves");
    assert_eq!(states, vec![UsJurisdiction::Wisconsin]);
}

#[test]
fn an_explicit_list_is_kept_in_caller_order() {
    let asked = vec![UsJurisdiction::Ohio, UsJurisdiction::Iowa];
    let states = resolve_states(false, &asked).expect("the list resolves");
    assert_eq!(states, asked);
}

#[test]
fn combining_the_two_flags_is_refused() {
    let error = resolve_states(true, &[UsJurisdiction::Ohio]).expect_err("both flags refused");
    assert!(error.to_string().contains("--all-states"));
}

#[test]
fn a_restriction_with_no_flag_is_empty_not_wisconsin() {
    let states = resolve_restriction(false, &[]).expect("no restriction");
    assert!(states.is_empty());
    let all = resolve_restriction(true, &[]).expect("--all-states resolves");
    assert_eq!(all, UsJurisdiction::CENSUS_SCOPE);
    let explicit = resolve_restriction(false, &[UsJurisdiction::Ohio]).expect("explicit");
    assert_eq!(explicit, vec![UsJurisdiction::Ohio]);
    assert!(resolve_restriction(true, &[UsJurisdiction::Ohio]).is_err());
}

use super::verify::{run_verify, VerifyArgs};
use census_domain::model::{CanonicalSchool, Evidence, SourceRef};
use census_store::{Store, Table};

#[test]
fn verification_uses_the_frozen_generation_after_the_store_changes() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let store = Store::open(directory.path().join("store")).expect("own store");
    let workbook = census_report::workbook::build(&store, &Default::default()).expect("publish");
    let (mut school, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Later School", "later school");
    school.evidence.push(Evidence::parsed(
        SourceRef::new(
            "wiaa_results",
            Some("https://example.test/results".to_string()),
        ),
        "2026-06-01",
    ));
    store
        .append(Table::Schools, &school)
        .expect("change live input");
    run_verify(
        store.root(),
        &VerifyArgs {
            workbook: Some(workbook),
        },
    )
    .expect("verify captured generation");
}

#[test]
fn verification_refuses_a_changed_artifact_in_the_published_bundle() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let store = Store::open(directory.path().join("store")).expect("own store");
    let workbook = census_report::workbook::build(&store, &Default::default()).expect("publish");
    std::fs::write(&workbook, "damaged workbook").expect("inject corruption");
    let error = run_verify(
        store.root(),
        &VerifyArgs {
            workbook: Some(workbook),
        },
    )
    .expect_err("refuse corruption");
    assert!(format!("{error:#}").contains("generation artifact mismatch: workbook.xlsx"));
}

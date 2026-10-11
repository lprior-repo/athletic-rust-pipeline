mod common;

use census_domain::UsJurisdiction;
use census_reconcile::identity::{admitted_scope, Revision, WorkflowIdentity, MAX_IDENTITY_BYTES};
use common::season;

#[test]
fn national_identity_is_generation_not_calendar() {
    let scope = [UsJurisdiction::Wisconsin];
    let first = WorkflowIdentity::national(season(), Revision(1), &scope);
    let again = WorkflowIdentity::national(season(), Revision(1), &scope);
    assert_eq!(first.as_str(), "national:2026-27:8ee4f7269711dd03:1");
    assert_eq!(first, again);
    assert_eq!(first.as_str().matches(':').count(), 3);
    assert_eq!(Revision(7).get(), 7);
    assert_eq!(Revision(7).to_string(), "7");
}

#[test]
fn revision_and_scope_change_the_run() {
    let scope = [UsJurisdiction::Wisconsin];
    let first = WorkflowIdentity::national(season(), Revision(1), &scope);
    assert_ne!(
        first,
        WorkflowIdentity::national(season(), Revision(2), &scope)
    );
    assert_ne!(
        first,
        WorkflowIdentity::national(season(), Revision(1), &[UsJurisdiction::Iowa])
    );
    assert_eq!(
        WorkflowIdentity::jurisdiction(UsJurisdiction::Wisconsin, season(), Revision(1)).as_str(),
        "jurisdiction:WI:2026-27:1"
    );
}

#[test]
fn scope_set_semantics_hold_over_order_and_duplication() {
    let forward = [UsJurisdiction::Iowa, UsJurisdiction::Wisconsin];
    let backward = [UsJurisdiction::Wisconsin, UsJurisdiction::Iowa];
    assert_eq!(
        WorkflowIdentity::national(season(), Revision(1), &forward),
        WorkflowIdentity::national(season(), Revision(1), &backward)
    );
    assert_eq!(
        WorkflowIdentity::national(season(), Revision(1), &forward),
        WorkflowIdentity::national(
            season(),
            Revision(1),
            &[
                UsJurisdiction::Iowa,
                UsJurisdiction::Iowa,
                UsJurisdiction::Wisconsin
            ]
        )
    );
    assert_eq!(
        WorkflowIdentity::national(season(), Revision(1), &[]).as_str(),
        WorkflowIdentity::national(season(), Revision(1), &UsJurisdiction::CENSUS_SCOPE).as_str()
    );
    assert_eq!(admitted_scope(&[]), UsJurisdiction::CENSUS_SCOPE.to_vec());
    assert_eq!(
        admitted_scope(&forward),
        vec![UsJurisdiction::Iowa, UsJurisdiction::Wisconsin]
    );
}

#[test]
fn jurisdiction_identities_fit_ceiling() {
    for jurisdiction in UsJurisdiction::ALL {
        let identity = WorkflowIdentity::jurisdiction(jurisdiction, season(), Revision(u32::MAX));
        assert!(
            identity.as_str().len() <= MAX_IDENTITY_BYTES,
            "{} exceeds the ceiling",
            jurisdiction.code()
        );
    }
    let full =
        WorkflowIdentity::national(season(), Revision(u32::MAX), &UsJurisdiction::CENSUS_SCOPE);
    assert!(full.as_str().len() <= MAX_IDENTITY_BYTES);
}

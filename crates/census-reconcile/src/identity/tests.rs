//! Identity behavior: the wire form, the purity that makes a retry reuse the same address, and the
//! two bounds that keep a provider's id space from deciding how long an identity gets.

use super::{scope_digest, Revision, WorkflowIdentity, MAX_IDENTITY_BYTES, MAX_PART_BYTES};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

/// The season the 2026-2027 census runs in, as the objective's `{season}` field spells it.
fn season() -> SchoolYear {
    SchoolYear::new(2026).expect("2026 is a season")
}

#[test]
fn national_identity_matches_the_documented_form() {
    let wisconsin = [UsJurisdiction::Wisconsin];
    let identity = WorkflowIdentity::national(season(), Revision(1), &wisconsin);
    assert_eq!(identity.as_str(), "national:2026-27:8ee4f7269711dd03:1");
    assert_eq!(
        identity.as_str().matches(':').count(),
        3,
        "season, run scope and revision are three fields"
    );
    assert_ne!(
        identity,
        WorkflowIdentity::national(season(), Revision(2), &wisconsin),
        "a revision bump must address a different national run"
    );
    assert_ne!(
        identity,
        WorkflowIdentity::national(season(), Revision(1), &[UsJurisdiction::Iowa]),
        "another state is another run"
    );
}

/// The scope is what a re-submission is checked against, so it has to be *in* the identity rather than
/// in an operator's memory: a revision already journalled under one jurisdiction set must not absorb a
/// submission naming another, and the same set must reproduce its identity byte for byte, which is what
/// lets a retry attach instead of duplicating a fan-out.
#[test]
fn a_changed_scope_is_a_changed_run_and_the_same_scope_reproduces_its_identity() {
    let both = [UsJurisdiction::Iowa, UsJurisdiction::Wisconsin];
    let scoped = WorkflowIdentity::national(season(), Revision(1), &both);

    assert_ne!(
        scoped,
        WorkflowIdentity::national(season(), Revision(1), &[UsJurisdiction::Wisconsin]),
        "a run over two states is not the run over one of them"
    );
    assert_eq!(
        scoped.as_str(),
        WorkflowIdentity::national(season(), Revision(1), &both).as_str(),
        "the same scope must reproduce the same run byte for byte"
    );
    assert_eq!(
        WorkflowIdentity::national(season(), Revision(1), &[]).as_str(),
        WorkflowIdentity::national(season(), Revision(1), &UsJurisdiction::CENSUS_SCOPE).as_str(),
        "an unnamed scope is the census run scope, spelled either way"
    );
}

/// The digest is taken in `CENSUS_SCOPE` order, over the set: naming the same states in another order
/// cannot move an existing run to a new identity, while adding, removing or replacing a state must.
#[test]
fn the_scope_digest_is_order_stable_over_a_set() {
    let forward = [
        UsJurisdiction::Iowa,
        UsJurisdiction::Wisconsin,
        UsJurisdiction::Ohio,
    ];
    let mut reversed = forward;
    reversed.reverse();

    assert_eq!(
        scope_digest(&forward),
        scope_digest(&reversed),
        "the order a caller names states in is not part of the census"
    );
    assert_eq!(
        scope_digest(&[UsJurisdiction::Iowa, UsJurisdiction::Iowa]),
        scope_digest(&[UsJurisdiction::Iowa])
    );
    assert_ne!(
        scope_digest(&forward),
        scope_digest(&[UsJurisdiction::Iowa, UsJurisdiction::Wisconsin]),
        "a set with a state removed is a different set"
    );
    assert_ne!(
        scope_digest(&[UsJurisdiction::Alaska]),
        scope_digest(&[]),
        "an inadmissible set does not collapse onto the run scope"
    );
}

/// The scope field is a fixed-width digest, so the whole identity stays inside the ceiling a store key
/// can carry however many states a caller names.
#[test]
fn a_full_scope_identity_fits_the_ceiling() {
    for jurisdictions in [
        UsJurisdiction::CENSUS_SCOPE.as_slice(),
        &[UsJurisdiction::Alaska],
    ] {
        let identity = WorkflowIdentity::national(season(), Revision(u32::MAX), jurisdictions);
        assert!(
            identity.as_str().len() <= MAX_IDENTITY_BYTES,
            "{} bytes over {} jurisdictions",
            identity.as_str().len(),
            jurisdictions.len()
        );
    }
}

#[test]
fn jurisdiction_identity_matches_the_documented_form() {
    let identity = WorkflowIdentity::jurisdiction(UsJurisdiction::Wisconsin, season(), Revision(1));
    assert_eq!(identity.as_str(), "jurisdiction:WI:2026-27:1");
    assert_eq!(identity.to_string(), "jurisdiction:WI:2026-27:1");
}

#[test]
fn identity_is_a_pure_function_of_its_fields() {
    let first = WorkflowIdentity::jurisdiction(UsJurisdiction::Alabama, season(), Revision(1));
    let again = WorkflowIdentity::jurisdiction(UsJurisdiction::Alabama, season(), Revision(1));
    assert_eq!(first, again, "the same fields must address the same work");

    let next_revision =
        WorkflowIdentity::jurisdiction(UsJurisdiction::Alabama, season(), Revision(2));
    assert_ne!(
        first, next_revision,
        "a revision bump must address different work, or the operator cannot invalidate a run"
    );

    let next_season = WorkflowIdentity::jurisdiction(
        UsJurisdiction::Alabama,
        SchoolYear::new(2027).expect("2027 is a season"),
        Revision(1),
    );
    assert_ne!(first, next_season);
}

#[test]
fn source_sweep_identity_separates_state_and_source() {
    let milesplit_wi = WorkflowIdentity::source_sweep(
        "milesplit",
        UsJurisdiction::Wisconsin,
        season(),
        Revision(1),
    );
    assert_eq!(milesplit_wi.as_str(), "source-sweep:milesplit:WI:2026-27:1");

    let milesplit_dc = WorkflowIdentity::source_sweep(
        "milesplit",
        UsJurisdiction::DistrictOfColumbia,
        season(),
        Revision(1),
    );
    let athleticnet_wi = WorkflowIdentity::source_sweep(
        "athleticnet",
        UsJurisdiction::Wisconsin,
        season(),
        Revision(1),
    );
    assert_ne!(milesplit_wi, milesplit_dc);
    assert_ne!(milesplit_wi, athleticnet_wi);
}

#[test]
fn an_oversized_provider_id_is_digested_not_truncated() {
    let long = "m".repeat(MAX_PART_BYTES.saturating_add(40));
    let other = format!("{long}x");
    let first = WorkflowIdentity::athlete("milesplit", &long, Revision(1));
    let second = WorkflowIdentity::athlete("milesplit", &other, Revision(1));

    assert!(first.as_str().len() <= MAX_IDENTITY_BYTES);
    assert!(second.as_str().len() <= MAX_IDENTITY_BYTES);
    assert_ne!(
        first, second,
        "two provider ids sharing a long prefix must not collapse onto one athlete"
    );
}

#[test]
fn a_separator_inside_a_field_cannot_forge_field_structure() {
    let forged = WorkflowIdentity::meet("athleticnet", "a:b", Revision(1));
    let plain = WorkflowIdentity::meet("athleticnet", "a", Revision(1));
    let shifted = WorkflowIdentity::meet("athleticnet:a", "b", Revision(1));

    assert_ne!(forged, plain);
    assert_ne!(
        forged, shifted,
        "the source and the meet id must not swap roles"
    );
    assert_eq!(
        forged.as_str().matches(':').count(),
        3,
        "a field carrying the separator must not add a fourth level"
    );
}

#[test]
fn an_empty_field_is_digested_rather_than_dropped() {
    let missing = WorkflowIdentity::school("wiaa", "", Revision(1));
    assert!(
        missing.as_str().len() > "school:wiaa:".len(),
        "an absent school id must still occupy its field"
    );
    assert_ne!(missing, WorkflowIdentity::school("wiaa", "-", Revision(1)));
}

#[test]
fn review_identity_keys_on_evidence_and_policy() {
    let digest = "9f2c4d";
    let policy = Revision(2);
    let review = WorkflowIdentity::review(digest, policy);
    assert_eq!(review.as_str(), "review:9f2c4d:2");
    assert_ne!(review, WorkflowIdentity::review("9f2c4e", policy));
    assert_ne!(review, WorkflowIdentity::review(digest, Revision(3)));
}

#[test]
fn every_jurisdiction_fits_the_identity_ceiling() {
    for jurisdiction in UsJurisdiction::ALL {
        let sweep = WorkflowIdentity::source_sweep(
            "coach-directories",
            jurisdiction,
            season(),
            Revision(u32::MAX),
        );
        assert!(
            sweep.as_str().len() <= MAX_IDENTITY_BYTES,
            "{} exceeds the ceiling: {} bytes for {}",
            jurisdiction.code(),
            sweep.as_str().len(),
            sweep.as_str()
        );
    }
}

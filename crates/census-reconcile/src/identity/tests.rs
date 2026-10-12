use super::{scope_digest, Revision, WorkflowIdentity, MAX_IDENTITY_BYTES};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn season() -> TestResult<SchoolYear> {
    Ok(SchoolYear::new(2026).ok_or("2026 is a season")?)
}

#[test]
fn national_identity_matches_the_documented_form() -> TestResult {
    let wisconsin = [UsJurisdiction::Wisconsin];
    let identity = WorkflowIdentity::national(season()?, Revision(1), &wisconsin);
    check!(eq; identity.as_str(), "national:2026-27:8ee4f7269711dd03:1");
    check!(eq; identity.as_str().matches(':').count(), 3, "season, run scope and revision are three fields");
    check!(ne; identity, WorkflowIdentity::national(season()?, Revision(2), &wisconsin), "a revision bump must address a different national run");
    check!(ne; identity, WorkflowIdentity::national(season()?, Revision(1), &[UsJurisdiction::Iowa]), "another state is another run");
    Ok(())
}

#[test]
fn a_changed_scope_is_a_changed_run_and_the_same_scope_reproduces_its_identity() -> TestResult {
    let both = [UsJurisdiction::Iowa, UsJurisdiction::Wisconsin];
    let scoped = WorkflowIdentity::national(season()?, Revision(1), &both);
    check!(ne; scoped, WorkflowIdentity::national(season()?, Revision(1), &[UsJurisdiction::Wisconsin]), "a run over two states is not the run over one of them");
    check!(eq; scoped.as_str(), WorkflowIdentity::national(season()?, Revision(1), &both).as_str(), "the same scope must reproduce the same run byte for byte");
    check!(eq; WorkflowIdentity::national(season()?, Revision(1), &[]).as_str(), WorkflowIdentity::national(season()?, Revision(1), &UsJurisdiction::CENSUS_SCOPE).as_str(), "an unnamed scope is the census run scope, spelled either way");
    Ok(())
}

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

#[test]
fn a_full_scope_identity_fits_the_ceiling() -> TestResult {
    for jurisdictions in [
        UsJurisdiction::CENSUS_SCOPE.as_slice(),
        &[UsJurisdiction::Alaska],
    ] {
        let identity = WorkflowIdentity::national(season()?, Revision(u32::MAX), jurisdictions);
        check!(
            identity.as_str().len() <= MAX_IDENTITY_BYTES,
            "{} bytes over {} jurisdictions",
            identity.as_str().len(),
            jurisdictions.len()
        );
    }
    Ok(())
}

#[test]
fn jurisdiction_identity_matches_the_documented_form() -> TestResult {
    let identity =
        WorkflowIdentity::jurisdiction(UsJurisdiction::Wisconsin, season()?, Revision(1));
    check!(eq; identity.as_str(), "jurisdiction:WI:2026-27:1");
    check!(eq; identity.to_string(), "jurisdiction:WI:2026-27:1");
    Ok(())
}

#[test]
fn identity_is_a_pure_function_of_its_fields() -> TestResult {
    let first = WorkflowIdentity::jurisdiction(UsJurisdiction::Alabama, season()?, Revision(1));
    let again = WorkflowIdentity::jurisdiction(UsJurisdiction::Alabama, season()?, Revision(1));
    check!(eq; first, again, "the same fields must address the same work");
    let next_revision =
        WorkflowIdentity::jurisdiction(UsJurisdiction::Alabama, season()?, Revision(2));
    check!(ne; first, next_revision, "a revision bump must address different work, or the operator cannot invalidate a run");
    let next_season = WorkflowIdentity::jurisdiction(
        UsJurisdiction::Alabama,
        SchoolYear::new(2027).ok_or("2027 is a season")?,
        Revision(1),
    );
    check!(ne; first, next_season);
    Ok(())
}

#[test]
fn every_jurisdiction_fits_the_identity_ceiling() -> TestResult {
    for jurisdiction in UsJurisdiction::ALL {
        let identity = WorkflowIdentity::jurisdiction(jurisdiction, season()?, Revision(u32::MAX));
        check!(
            identity.as_str().len() <= MAX_IDENTITY_BYTES,
            "{} exceeds the ceiling: {} bytes for {}",
            jurisdiction.code(),
            identity.as_str().len(),
            identity.as_str()
        );
    }
    Ok(())
}

#[test]
fn a_resume_addresses_a_new_parent_for_the_same_run_and_scope() -> TestResult {
    let states = [UsJurisdiction::Wisconsin, UsJurisdiction::Iowa];
    let ordinal = std::num::NonZeroU32::new(7).ok_or("fixture ordinal")?;
    let resumed = WorkflowIdentity::national_resume(season()?, Revision(1), &states, ordinal);
    let original = WorkflowIdentity::national(season()?, Revision(1), &states);
    check!(eq; resumed.as_str(), format!("{}:resume:7", original.as_str()));
    let reversed = [
        UsJurisdiction::Iowa,
        UsJurisdiction::Wisconsin,
        UsJurisdiction::Iowa,
    ];
    check!(eq; resumed, WorkflowIdentity::national_resume(season()?, Revision(1), &reversed, ordinal));
    check!(ne; resumed, WorkflowIdentity::national_resume(season()?, Revision(1), &states, std::num::NonZeroU32::new(8).ok_or("next ordinal")?));
    check!(ne; resumed, WorkflowIdentity::national_resume(season()?, Revision(2), &states, ordinal));
    check!(ne; resumed, WorkflowIdentity::national_resume(season()?, Revision(1), &[UsJurisdiction::Iowa], ordinal));
    Ok(())
}

#[test]
fn the_maximum_resume_ordinal_is_not_truncated_and_fits_the_identity_ceiling() -> TestResult {
    let ordinal = std::num::NonZeroU32::new(u32::MAX).ok_or("maximum ordinal")?;
    for year in [SchoolYear::MIN_START_YEAR, SchoolYear::MAX_START_YEAR] {
        let school_year = SchoolYear::new(year).ok_or("boundary season")?;
        for states in [
            UsJurisdiction::CENSUS_SCOPE.as_slice(),
            &[UsJurisdiction::Alaska],
        ] {
            let identity =
                WorkflowIdentity::national_resume(school_year, Revision(u32::MAX), states, ordinal);
            check!(identity.as_str().len() <= MAX_IDENTITY_BYTES);
            check!(identity.as_str().ends_with(":4294967295:resume:4294967295"));
        }
    }
    Ok(())
}

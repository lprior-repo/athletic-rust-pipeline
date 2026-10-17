use super::*;
use census_domain::model::AthleteCandidateId;
use census_domain::public_identity_history::{PublicAthleteId, PublicResolution};
use census_store::PublicIdentityWriterV1;

mod control;
mod operations;
mod oracle;
mod review;
mod stage;

#[derive(Clone, Copy)]
enum Branch {
    Split,
    RevokeRedundantMerge,
}

#[test]
fn controlled_reviewed_roster_split_changes_nonempty_publication_identity_and_preserves_g0(
) -> TestResult {
    run(Branch::Split)
}

#[test]
fn controlled_reviewed_roster_redundant_merge_revocation_preserves_nonempty_publication_and_g0(
) -> TestResult {
    run(Branch::RevokeRedundantMerge)
}

fn run(branch: Branch) -> TestResult {
    let (fixture, expected) = setup::prepare(Case::Resolved)?;
    let controlled = control::install(&fixture.store, &fixture.writer)?;
    let objects = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let root = output.path().join("publication");
    let claims = oracle::source_claims(&fixture.store)?;
    let context = oracle::source_context(&fixture.store)?;
    let g0 = stage::Stage::capture(
        &fixture,
        &expected,
        &controlled.public,
        objects.path(),
        &root,
    )?;
    let merged = operations::merge(&fixture.store, &fixture.writer, &controlled)?;
    stage::stale(&g0.dataset, &fixture.store)?;
    g0.unchanged()?;
    check!(eq; oracle::source_claims(&fixture.store)?, claims);
    check!(eq; oracle::source_context(&fixture.store)?, context);
    let merged_ids = vec![controlled.public[0].clone(), controlled.public[0].clone()];
    let g1 = stage::Stage::capture(&fixture, &expected, &merged_ids, objects.path(), &root)?;
    check!(eq; g1.dataset.verified_public_identities()?.head().sequence, merged);
    let corrected =
        operations::correct(&fixture.store, &fixture.writer, &controlled, merged, branch)?;
    stage::stale(&g1.dataset, &fixture.store)?;
    g0.unchanged()?;
    g1.unchanged()?;
    let ids = oracle::corrected(&fixture.store, &controlled, corrected, merged, branch)?;
    check!(eq; oracle::source_claims(&fixture.store)?, claims);
    check!(eq; oracle::source_context(&fixture.store)?, context);
    let g2 = stage::Stage::capture(&fixture, &expected, &ids, objects.path(), &root)?;
    check!(eq; g2.dataset.verified_public_identities()?.head().sequence, corrected);
    check!(ne; g2.resolutions, g1.resolutions);
    match branch {
        Branch::Split => check!(ne; g2.public, g1.public),
        Branch::RevokeRedundantMerge => check!(eq; g2.public, g1.public),
    }
    check!(eq; published::current_workbook(&root)?, g2.workbook);
    let retained = [g0.freeze()?, g1.freeze()?, g2.freeze()?];
    remove_sources(fixture, objects)?;
    for generation in retained {
        generation.replay(&expected)?;
    }
    Ok(())
}

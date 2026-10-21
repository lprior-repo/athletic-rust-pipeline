use super::*;
use crate::participation_evidence::packet::roncalli::tests as roncalli_fixture;

mod calendar_candidates;
mod correspondence;
mod material;
mod observe;
mod provider;
mod review;
mod source;

#[test]
fn genuine_tessa_native_identity_result_and_exact_day_roncalli_attachment_replay_frozen_and_cold(
) -> TestResult {
    let installed = source::install()?;
    let result = review::attach(&installed, false)?;
    let status = review::attach(&installed, true)?;
    source::originals(&installed.store, &installed.originals)?;
    let original = observe::current(&installed.store, &installed.athlete, &installed.school)?;
    let bundle = super::observe::freeze(&installed.store)?;
    let frozen_identity = installed
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("frozen native Tessa identity")?;
    check!(eq; observe::frozen(&frozen_identity, &bundle, &installed.athlete, &installed.school)?, original);
    review::replay(&installed.store, result.0.clone(), &result.1)?;
    review::replay(&installed.store, status.0.clone(), &status.1)?;
    check!(eq; observe::current(&installed.store, &installed.athlete, &installed.school)?, original);
    let protected = state(&installed.store)?;
    let source::Installed {
        directory,
        store,
        school,
        athlete,
        public,
        school_reference,
        participation,
        context,
        originals,
    } = installed;
    store.flush()?;
    drop(store);
    check!(eq; observe::frozen(&frozen_identity, &bundle, &athlete, &school)?, original);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("store"))?.into_parts();
    drop(writer);
    check!(eq; state(&store)?, protected);
    source::originals(&store, &originals)?;
    check!(eq; observe::current(&store, &athlete, &school)?, original);
    check!(eq; observe::frozen(&frozen_identity, &bundle, &athlete, &school)?, original);
    review::replay(&store, result.0, &result.1)?;
    review::replay(&store, status.0, &status.1)?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let reopened = crate::participation_evidence::recorded::read_recorded_provider_school(
        &snapshot,
        &school_reference,
        &mut grant,
    )?
    .ok_or("cold actual Roncalli school")?;
    check!(eq; reopened.checked().school().id, school.id);
    let reopened =
        crate::participation_evidence::recorded::read_recorded_independent_participation(
            &snapshot,
            &participation,
            &mut grant,
        )?
        .ok_or("cold actual Roncalli pair")?;
    check!(eq; reopened.checked().cohort(), GradYear::CO2027);
    let reopened =
        read_recorded_tfrrs_list_capture(&snapshot, &context, &[source::locator()], &mut grant)?
            .ok_or("cold actual Tessa list")?;
    check!(eq; reopened.read(&[source::locator()])?.row(0).ok_or("cold original Tessa row")?.athlete_id(), "8779607");
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("cold native public identity")?;
    check!(
        matches!(accepted.resolve_candidate(&athlete.id.cast())?, census_domain::public_identity_history::PublicResolution::Canonical(row) if row.public_id == public)
    );
    check!(eq; state(&store)?, protected);
    Ok(())
}

fn state(store: &Store) -> TestResult<Vec<u8>> {
    let snapshot = store.snapshot();
    Ok(serde_json::to_vec(&(
        snapshot.sequence(),
        snapshot.evidence_generation(),
        snapshot.derived_generation(),
        store.receipt_count()?,
        snapshot.tables_digest(&Table::ALL)?,
        format!("{:?}", snapshot.public_identity_input_revisions_v2()?),
        snapshot
            .accepted_public_identity_snapshot()?
            .ok_or("current accepted Tessa identity")?
            .head(),
    ))?)
}

use super::fixture::{merge, projection, source_births, Fixture, TestResult};
use super::VerifiedPublicIdentityView;
use crate::export::store_lineage;
use census_domain::public_identity_history::{PublicAthleteId, PublicResolution};
use census_store::Store;

fn inventory(
    view: &VerifiedPublicIdentityView,
) -> TestResult<Vec<(PublicAthleteId, PublicResolution)>> {
    view.public_ids()
        .map(|id| Ok((id.clone(), view.resolve_public(id)?)))
        .collect()
}

#[test]
fn historical_ids_survive_absent_candidate_groups_reopen_and_frozen_replay() -> TestResult {
    let fixture = source_births()?;
    let merged = merge(&fixture)?;
    let snapshot = fixture.store.snapshot();
    let lineage = store_lineage(&fixture.store)?;
    let projection = projection(&snapshot)?;
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted public history")?;
    let frozen = snapshot.freeze_public_identity_v1()?;
    let view =
        VerifiedPublicIdentityView::capture(accepted, &lineage, merged.head(), &[], &projection)?;
    let [first_birth, second_birth] = &fixture.births;
    let mut expected_ids = vec![first_birth.public_id(), second_birth.public_id()];
    expected_ids.sort();
    check!(eq; view.public_ids().collect::<Vec<_>>(), expected_ids);
    for athlete in &fixture.athletes {
        check!(view.group(athlete.id.as_str()).is_none());
        check!(view.public_id(athlete.id.as_str()).is_none());
    }
    check!(matches!(
        view.resolve_public(first_birth.public_id())?,
        PublicResolution::Canonical(_)
    ));
    check!(matches!(
        view.resolve_public(second_birth.public_id())?,
        PublicResolution::Redirect { .. }
    ));
    let expected = inventory(&view)?;
    check!(eq; fixture.store.snapshot().sequence(), lineage.snapshot_sequence);
    drop(snapshot);
    let Fixture {
        directory,
        store,
        writer,
        ..
    } = fixture;
    drop(writer);
    drop(store);
    let (store, _writer) =
        Store::open_public_identity_qualification_v2(directory.path().join("identity"))?
            .into_parts();
    let snapshot = store.snapshot();
    let reopened_lineage = store_lineage(&store)?;
    let reopened = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("reopened public history")?;
    let reopened_view = VerifiedPublicIdentityView::capture(
        reopened,
        &reopened_lineage,
        merged.head(),
        &[],
        &projection,
    )?;
    check!(eq; inventory(&reopened_view)?, expected);
    let replayed_view = VerifiedPublicIdentityView::capture(
        frozen.replay()?,
        &lineage,
        merged.head(),
        &[],
        &projection,
    )?;
    check!(eq; inventory(&replayed_view)?, expected);
    check!(eq; inventory(&view)?, expected);
    Ok(())
}

use crate::{PublicIdentityError, StoreSnapshot};
use census_domain::model::AthleteIdentityIndex;
use census_domain::public_identity_history::PublicResolution;

use super::support::refused;
use super::*;

#[test]
fn public_identity_legacy_projection_entrypoints_refuse_empty_and_unestablished_stores(
) -> TestResult {
    let (_directory, store, writer) = fixture()?;
    check!(eq; store.store_format()?.lifecycle_revision, Some(7));
    check!(eq; writer.head(&store)?.sequence, 0);
    refused_entrypoints(&store, &store.snapshot())?;
    let athlete = subject("unestablished-projection", "704")?;
    store.append(Table::Athletes, &athlete)?;
    let snapshot = store.snapshot();
    check!(eq; snapshot.athlete_identity_index()?.subjects().count(), 1);
    check!(eq; writer.head(&store)?.sequence, 0);
    refused_entrypoints(&store, &snapshot)?;
    Ok(())
}

#[test]
fn public_identity_legacy_projection_entrypoints_refuse_established_and_pinned_snapshots(
) -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("established-projection", "705")?;
    store.append(Table::Athletes, &athlete)?;
    let held = store.snapshot();
    refused_entrypoints(&store, &held)?;
    let committed = writer.establish_source_bound(
        &store,
        "established-projection",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(!committed.replayed());
    check!(eq; committed.head().sequence, 1);
    let current = store.snapshot();
    let accepted = current
        .accepted_public_identity_snapshot()?
        .ok_or("established snapshot lacks accepted ledger")?;
    let PublicResolution::Canonical(resolved) = accepted.resolve_candidate(&athlete.id.cast())?
    else {
        return Err("established candidate lacks accepted public identity".into());
    };
    check!(eq; &resolved.public_id, committed.public_id());
    refused_entrypoints(&store, &held)?;
    refused_entrypoints(&store, &current)?;
    Ok(())
}

fn refused_entrypoints(store: &Store, snapshot: &StoreSnapshot<'_>) -> TestResult {
    let before = retained_state(store)?;
    check!(eq; before.len(), 4);
    check!(matches!(
        refused(store.athlete_identity_projection())?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(eq; retained_state(store)?, before);
    check!(matches!(
        refused(snapshot.athlete_identity_projection())?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(eq; retained_state(store)?, before);
    let empty = AthleteIdentityIndex::default();
    check!(empty.subjects().next().is_none());
    check!(matches!(
        refused(snapshot.project_athlete_identities(empty))?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(eq; retained_state(store)?, before);
    let retained = snapshot.athlete_identity_index()?;
    check!(matches!(
        refused(snapshot.project_athlete_identities(retained))?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(eq; retained_state(store)?, before);
    Ok(())
}

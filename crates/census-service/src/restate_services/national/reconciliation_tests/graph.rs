use super::{AppliedAthleteIdentity, CanonicalAthlete, Store, Table, TestResult};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use census_store::{AcceptedPublicIdentitySnapshot, PublicIdentityError, StoreError};

pub(super) fn read(store: &Store) -> TestResult<AcceptedPublicIdentitySnapshot> {
    Ok(store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("accepted Graph absent")?)
}

pub(super) fn unchanged(
    store: &Store,
    before: &AcceptedPublicIdentitySnapshot,
    candidates: &[CanonicalAthlete],
) -> TestResult {
    let after = read(store)?;
    check!(eq; after.qualification_version(), PublicIdentityQualificationVersion::RetainedAuthorityGraphV3);
    check!(eq; after.store_identity(), before.store_identity());
    check!(eq; after.head(), before.head());
    check!(eq; after.history_records(), before.history_records());
    check!(eq; after.public_ids().collect::<Vec<_>>(), before.public_ids().collect::<Vec<_>>());
    for id in before.public_ids() {
        check!(eq; after.resolve_public(id)?, before.resolve_public(id)?);
    }
    for candidate in candidates {
        check!(eq; after.resolve_candidate(&candidate.id.cast())?, before.resolve_candidate(&candidate.id.cast())?);
    }
    Ok(())
}

pub(super) fn state(store: &Store) -> TestResult<(u64, u64, u64, u64, String)> {
    Ok((
        store.snapshot().sequence(),
        store.receipt_count()?,
        store.evidence_generation(),
        store.derived_generation(),
        store.snapshot().tables_digest(&Table::ALL)?,
    ))
}

pub(super) fn legacy_write_refused(store: &Store) -> TestResult {
    let before = read(store)?;
    let rows = state(store)?;
    check!(matches!(
        store.replace_many::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions, &[]),
        Err(StoreError::PublicIdentity(
            PublicIdentityError::ReservedWrite
        ))
    ));
    check!(matches!(
        store.athlete_identity_projection(),
        Err(StoreError::PublicIdentity(
            PublicIdentityError::OptInRequired
        ))
    ));
    unchanged(store, &before, &[])?;
    check!(eq; state(store)?, rows);
    Ok(())
}

pub(super) fn summary(store: &Store, journal: &super::Journal) -> TestResult {
    let entry = journal
        .entries
        .iter()
        .find(|entry| {
            entry.kind == 0x0403 && entry.text(1).is_ok_and(|key| key == "identity:reconcile")
        })
        .ok_or("selected reconciliation state absent")?;
    let bytes = super::super::publication_tests::codec::field(entry.bytes(3)?, 1)?
        .ok_or("selected reconciliation state value absent")?;
    let summary: String = serde_json::from_slice(bytes)?;
    let accepted = read(store)?;
    check!(summary.starts_with("selected_graph_v3 "));
    check!(summary.contains(&format!(
        "profile={}",
        journal
            .original
            .export_job
            .require_selected_publication()
            .profile_id()
    )));
    check!(summary.contains(&format!("head_sequence={}", accepted.head().sequence)));
    check!(summary.contains(&format!("head_digest={}", accepted.head().digest)));
    check!(summary.contains(&format!("public_ids={}", accepted.public_ids().count())));
    check!(summary.contains(&format!(
        "history_records={}",
        accepted.history_records().len()
    )));
    check!(summary.contains("source_identity_derivation=selected_workbook"));
    Ok(())
}

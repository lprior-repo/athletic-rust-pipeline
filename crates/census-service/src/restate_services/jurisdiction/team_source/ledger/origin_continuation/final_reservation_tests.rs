use super::*;

#[test]
fn final_unknown_reservation_is_settled_durably_before_status_reports_interrupted(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let (request, key, digest) = tests::original(&store)?;
    let authorization = authorize(&store, &key, &request, &digest)?;
    begin_at(&store, &key, &request, &authorization, 1)?;
    begin_at(&store, &key, &request, &authorization, 2)?;
    store.flush()?;
    drop(store);
    let store = Store::open(root.path())?;
    check!(status(&store, &key)?.is_none());
    check!(eq; super::super::durable_attempts(&store)?.settled, 0);
    check!(matches!(
        begin_at(&store, &key, &request, &authorization, 3)?,
        AdmissionReplay::Settled(TeamsSourceOutcome::Interrupted {
            attempts: Some(3),
            ..
        })
    ));
    check!(matches!(
        status(&store, &key)?,
        Some(TeamsSourceOutcome::Interrupted {
            attempts: Some(3),
            ..
        })
    ));
    let counts = super::super::durable_attempts(&store)?;
    check!(eq; (counts.reserved, counts.recorded, counts.unknown, counts.settled), (3, 1, 2, 1));
    check!(store.journal_contains(PHASE, &format!("{key}/settled"))?);
    check!(!store.journal_contains(PHASE, &slot_key(&key, 4, "reserved"))?);
    Ok(())
}

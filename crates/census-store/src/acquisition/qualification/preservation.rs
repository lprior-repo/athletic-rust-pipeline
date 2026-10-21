use super::*;

pub(crate) fn validate_preservation(
    root: &Path,
    db: &Database,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
) -> StoreResult<()> {
    let snapshot = db.snapshot();
    let lifecycle_size = snapshot
        .size_of(meta, crate::format::LIFECYCLE_REVISION_KEY)
        .map_err(|source| StoreError::Read { source })?;
    let raw_lifecycle = if lifecycle_size.is_some_and(|size| size <= 32) {
        snapshot
            .get(meta, crate::format::LIFECYCLE_REVISION_KEY)
            .map_err(|source| StoreError::Read { source })?
    } else {
        None
    };
    let lifecycle = raw_lifecycle
        .as_ref()
        .and_then(|value| std::str::from_utf8(value).ok())
        .and_then(|value| value.trim().parse::<u32>().ok());
    let has_marker = db.snapshot().prefix(meta, b"capture:").next().is_some();
    let has_family = db
        .snapshot()
        .prefix(journal, b"source_acquisition_attempt_")
        .next()
        .is_some();
    let has_receipt = snapshot
        .prefix(receipts, b"source_acquisition_attempt_")
        .next()
        .is_some();
    if lifecycle != Some(CAPTURE_QUALIFICATION_LIFECYCLE)
        && !has_marker
        && !has_family
        && !has_receipt
        && !crate::acquisition::public_resources::format::has_state(
            &snapshot, meta, journal, receipts,
        )
    {
        return Ok(());
    }
    let format = crate::format::require_current(root, crate::format::read_format(meta)?)?;
    validate_format(root, &db.snapshot(), meta, journal, receipts, &format)?;
    Ok(())
}

use super::*;
use read::lower_hex;

pub(super) fn header(
    schema: &str,
    store: &str,
    head: &Head<'_>,
    inventory: &Inventory<'_>,
) -> StoreResult<()> {
    if schema != SCHEMA
        || store.len() != 64
        || !store.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !digest(head.digest)
        || !digest(inventory.digest)
        || inventory.version != 1
    {
        return Err(Error::Corrupt("frozen archive header").into());
    }
    Ok(())
}

fn digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(lower_hex)
}

pub(super) fn root(kind: Kind, root: Option<&Root<'_>>) -> StoreResult<()> {
    let limit = u64::try_from(
        crate::evidence_subject_attachment::external::MAX_RETAINED_CAPTURE_ENVELOPE_BYTES,
    )
    .map_err(|_| StoreError::CounterOverflow)?;
    match (kind, root) {
        (Kind::Attachment | Kind::Receipts, None) => Ok(()),
        (Kind::External, Some(root))
            if digest(root.key)
                && digest(root.payload.digest)
                && root.payload.bytes > 0
                && root.payload.bytes <= limit =>
        {
            Ok(())
        }
        _ => Err(Error::Corrupt("frozen archive prefix descriptor").into()),
    }
}

pub(super) fn archive(
    archive: &Archive<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    steps(grant, 1)?;
    if archive.prefixes.len() < 2 {
        return Err(Error::Corrupt("frozen archive fixed prefixes").into());
    }
    let mut previous = None;
    for (index, group) in archive.prefixes.iter().enumerate() {
        steps(grant, 1)?;
        let expected = match index {
            0 => Kind::Attachment,
            1 => Kind::Receipts,
            _ => Kind::External,
        };
        if group.kind != expected {
            return Err(Error::Corrupt("frozen archive prefix order").into());
        }
        if let Some(root) = &group.root {
            let key = (root.schema.phase(), root.key);
            if previous.is_some_and(|old| old >= key) {
                return Err(Error::Corrupt("frozen archive duplicate or unordered prefix").into());
            }
            previous = Some(key);
            if group.rows.is_empty() {
                return Err(Error::Corrupt("frozen archive empty external prefix").into());
            }
        }
        rows(group, grant)?;
    }
    Ok(())
}

fn rows(group: &Prefix<'_>, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
    let mut previous: Option<&[u8]> = None;
    let mut total = 0usize;
    for row in &group.rows {
        steps(grant, 1)?;
        if previous.is_some_and(|old| old >= row.key.as_slice()) {
            return Err(Error::Corrupt("frozen archive duplicate or unordered row").into());
        }
        if !covers(group, &row.key) {
            return Err(Error::Corrupt("frozen archive row outside prefix").into());
        }
        if group.kind == Kind::Receipts {
            bound("frozen receipt key", row.key.len(), 512)?;
            bound("frozen receipt value", row.value.get().len(), 2048)?;
        }
        total = state::add(total, state::add(row.key.len(), row.value.get().len())?)?;
        previous = Some(&row.key);
    }
    if u64::try_from(group.rows.len()).map_err(|_| StoreError::CounterOverflow)?
        != group.physical_rows
        || u64::try_from(total).map_err(|_| StoreError::CounterOverflow)? != group.encoded_bytes
    {
        return Err(Error::Corrupt("frozen archive physical summary differs").into());
    }
    Ok(())
}

pub(super) fn covers(group: &Prefix<'_>, key: &[u8]) -> bool {
    match group.kind {
        Kind::Attachment => key
            .strip_prefix(PHASE.as_bytes())
            .is_some_and(|tail| tail.starts_with(&[0])),
        Kind::Receipts => key.starts_with(OPERATION_PREFIX.as_bytes()),
        Kind::External => group.root.as_ref().is_some_and(|root| {
            key.strip_prefix(root.schema.phase().as_bytes())
                .and_then(|tail| tail.strip_prefix(&[0]))
                .is_some_and(|tail| tail.starts_with(root.key.as_bytes()))
        }),
    }
}

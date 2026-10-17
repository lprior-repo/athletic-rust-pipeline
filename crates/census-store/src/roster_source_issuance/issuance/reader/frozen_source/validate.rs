use super::*;

pub(super) fn prefixes(prefixes: &[CapturedExternalPrefix], grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
    grant.ensure_available().map_err(domain)?;
    grant.charge_steps(1).map_err(domain)?;
    if prefixes.len() > 4096 { return Err(outside()); }
    let mut previous = None;
    for group in prefixes {
        grant.charge_steps(1).map_err(domain)?;
        grant.charge_bytes(group.root.key.len()).map_err(domain)?;
        let identity = (group.root.schema.phase(), group.root.key.as_str());
        if previous.is_some_and(|old| old >= identity) || group.root.key.len() != 64
            || !group.root.key.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || group.root.payload.bytes == 0 || group.root.payload.bytes > 2 * 1024 * 1024 {
            return Err(outside());
        }
        rows(group, grant)?;
        previous = Some(identity);
    }
    Ok(())
}

fn rows(group: &CapturedExternalPrefix, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
    let namespace = JournalNamespace::new(group.root.schema.phase(), &group.root.key)?;
    let budget = ReadBudget { rows: 6, bytes: 4 * 1024 * 1024, record_bytes: crate::MAX_JOURNAL_VALUE_BYTES };
    admit_namespace_budget(budget)?;
    let mut scan = NamespaceScan::new(namespace, budget)?;
    let mut previous: Option<&[u8]> = None;
    for row in &group.rows {
        grant.charge_steps(1).map_err(domain)?;
        let bytes = row.key.len().checked_add(row.value.len()).ok_or(StoreError::CounterOverflow)?;
        grant.charge_bytes(bytes).map_err(domain)?;
        if previous.is_some_and(|old| old >= row.key.as_slice()) || !covers(group, &row.key) {
            return Err(outside());
        }
        scan.charge_visit()?;
        scan.accept(&row.key, &row.value)?;
        previous = Some(&row.key);
    }
    let _contents = scan.finish();
    Ok(())
}

fn covers(group: &CapturedExternalPrefix, key: &[u8]) -> bool {
    key.strip_prefix(group.root.schema.phase().as_bytes())
        .and_then(|tail| tail.strip_prefix(&[0]))
        .is_some_and(|tail| tail.starts_with(group.root.key.as_bytes()))
}

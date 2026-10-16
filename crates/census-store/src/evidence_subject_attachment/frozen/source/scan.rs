use super::*;

pub(super) fn journal<G>(
    source: &FrozenRetainedCaptureSourceV1<'_>,
    namespace: JournalNamespace<'_>,
    budget: ReadBudget,
    state: &mut G,
    mut admit: impl FnMut(&mut G) -> StoreResult<()>,
    mut visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
) -> StoreResult<JournalNamespaceContents> {
    admit_namespace_budget(budget)?;
    let prefix = namespace.physical_prefix()?;
    let group = source.namespace(namespace)?;
    let mut scan = NamespaceScan::new(namespace, budget)?;
    for row in coverage::range(group, &prefix)? {
        admit(state)?;
        scan.charge_visit()?;
        scan.accept(&row.key, row.value.get().as_bytes())?;
        visit(state, &row.key, row.value.get().as_bytes())?;
    }
    Ok(scan.finish())
}

pub(super) fn receipts<G>(
    source: &FrozenRetainedCaptureSourceV1<'_>,
    prefix: &[u8],
    budget: ReadBudget,
    state: &mut G,
    mut admit: impl FnMut(&mut G) -> StoreResult<()>,
    mut visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
) -> StoreResult<ReadCounts> {
    let mut scan = ReceiptScan::new(prefix, budget)?;
    let group = source.receipts(prefix)?;
    for row in coverage::range(group, prefix)? {
        admit(state)?;
        scan.charge_visit()?;
        let raw = row.value.get().as_bytes();
        let raw = scan.read(
            &row.key,
            || {
                Ok(Some(
                    u32::try_from(raw.len()).map_err(|_| StoreError::CounterOverflow)?,
                ))
            },
            || Ok(Some(raw)),
        )?;
        visit(state, &row.key, raw)?;
    }
    Ok(scan.finish())
}

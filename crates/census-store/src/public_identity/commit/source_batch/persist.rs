use super::*;

pub(super) fn finish(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    request: &Request,
    expected: StoreRevision<'_>,
    original_head: &RegistryHead,
    pending: Pending,
) -> StoreResult<Vec<IdentityCommitOutcomeV1>> {
    let _guard = store.lock_appends();
    let ledger = writer.verified(store)?;
    if let Some(outcomes) = replay::replay(writer, store, request, &ledger)? {
        return Ok(outcomes);
    }
    graph_operation::require_current(
        writer,
        store,
        &request.operation,
        expected,
        original_head,
        &pending.capture.graph,
    )?;
    let count = u64::try_from(pending.items.len()).map_err(|_| StoreError::CounterOverflow)?;
    let rows = store
        .count(crate::Table::AthleteIdentityDecisions)?
        .checked_add(count)
        .ok_or(StoreError::CounterOverflow)?;
    crate::batch::refuse_over_bound(crate::Table::AthleteIdentityDecisions, rows)?;
    let generation = store
        .evidence_generation()
        .checked_add(count)
        .ok_or(StoreError::CounterOverflow)?;
    let mut batch = store.db.batch();
    let outcomes = stage_items(store, &mut batch, &pending)?;
    store.put_row_mark(&mut batch, crate::Table::AthleteIdentityDecisions, rows);
    batch.insert(&store.meta, HEAD, codec::encode(&pending.head, 1024)?);
    batch.insert(&store.meta, HISTORY, pending.history);
    let receipt = crate::Receipt {
        operation: request.operation.clone(),
        digest: request.digest.clone(),
        at: pending.capture.observed_at,
        appended: count,
        credited: false,
    };
    batch.insert(
        &store.receipts,
        &request.operation,
        codec::encode(&receipt, 65_536)?,
    );
    crate::generation::write_entities(&mut batch, &store.meta, generation);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    store.generations.publish_entities(generation);
    Ok(outcomes)
}

fn stage(
    store: &Store,
    batch: &mut fjall::OwnedWriteBatch,
    item: &Item,
    at: &str,
) -> StoreResult<()> {
    let limit = DecisionWrite::Accepted(item.accepted.record()).stage(store, batch)?;
    let operation = witness::receipt_key(item.witness.operation());
    let receipt = crate::Receipt {
        operation: operation.clone(),
        digest: serialized_digest(&item.witness)?,
        at: at.to_owned(),
        appended: 1,
        credited: false,
    };
    if codec::load(&store.db.snapshot(), &store.receipts, &operation, 65_536)?.is_some() {
        return Err(PublicIdentityError::OperationConflict.into());
    }
    batch.insert(
        &store.journal,
        witness::key(item.record.sequence),
        codec::encode(&item.witness, limit)?,
    );
    batch.insert(&store.receipts, operation, codec::encode(&receipt, 65_536)?);
    Ok(())
}

fn stage_items(
    store: &Store,
    batch: &mut fjall::OwnedWriteBatch,
    pending: &Pending,
) -> StoreResult<Vec<IdentityCommitOutcomeV1>> {
    let mut outcomes = Vec::new();
    let mut decisions = BTreeSet::new();
    for item in &pending.items {
        if !decisions.insert(&item.accepted.record().id) {
            return Err(PublicIdentityError::Corrupt("duplicate batch decision").into());
        }
        stage(store, batch, item, &pending.capture.observed_at)?;
        push(&mut outcomes, outcome(&item.witness, &item.record, false)?)?;
    }
    Ok(outcomes)
}

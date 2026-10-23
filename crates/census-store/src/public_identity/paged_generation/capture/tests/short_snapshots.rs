use super::*;

fn record(store: &Store, node: Node<'_>, encoded: &mut Vec<Vec<u8>>) -> StoreResult<ObjectRefV2> {
    let physical = node.measure(store)?.full_bytes();
    let (bytes, reference) = match node {
        Node::Page(value) => (value.encode_jsonl()?, value.object_reference(physical)?),
        Node::Block(value) => (value.encode_jsonl()?, value.object_reference(physical)?),
        Node::Table(value) => (value.encode_jsonl()?, value.object_reference(physical)?),
        Node::Seal(value) => (value.encode_jsonl()?, value.object_reference(physical)?),
    };
    encoded.push(bytes);
    Ok(reference)
}

#[test]
fn short_snapshots_preserve_every_raw_node_and_seal_across_journal_writes() -> TestResult {
    let (_directory, store, writer) = seeded_and_reopened(513)?;
    let authority = concurrency::authority_state(&store, &writer)?;
    let mut expected = Vec::new();
    let (capture, held_seal) = {
        let snapshot = store.snapshot();
        let capture = binding::capture(&store, &snapshot)?;
        let seal = scan::walk(scan::Input::Held(&snapshot), &capture, |node| {
            record(&store, node, &mut expected)
        })?;
        (capture, seal)
    };
    let sequence = store.db.visible_seqno();
    let mut actual = Vec::new();
    let mut pages = Vec::new();
    let current_seal = scan::walk(scan::Input::Current(&store), &capture, |node| {
        if let Node::Page(page) = &node {
            pages.push(page.rows().len());
        }
        let reference = record(&store, node, &mut actual)?;
        let _guard = store.lock_appends();
        store
            .journal
            .insert(
                "short-snapshot-journal-churn",
                actual.len().to_string().as_bytes(),
            )
            .map_err(|source| StoreError::Write { source })?;
        Ok(reference)
    })?;
    check!(eq;pages, [512, 1]);
    check!(eq;actual.len(), 9);
    check!(eq;actual, expected);
    check!(eq;current_seal.encode_jsonl()?, held_seal.encode_jsonl()?);
    check!(store.db.visible_seqno() > sequence);
    check!(eq;concurrency::authority_state(&store, &writer)?, authority);
    Ok(())
}

#[test]
fn short_snapshots_refuse_protected_mutation_between_raw_pages() -> TestResult {
    let (_directory, store, writer) = seeded_and_reopened(513)?;
    let authority = concurrency::authority_state(&store, &writer)?;
    let capture = binding::capture(&store, &store.snapshot())?;
    let mutation = subject("between capture pages", "99901")?;
    let mut encoded = Vec::new();
    let mut page_count = 0_u64;
    let result = scan::walk(scan::Input::Current(&store), &capture, |node| {
        if let Node::Page(_) = &node {
            page_count = page_count
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?;
            store.append(Table::Athletes, &mutation)?;
        }
        record(&store, node, &mut encoded)
    });
    check!(matches!(&result, Err(StoreError::Refused { detail })
        if detail == "raw public identity generation V2: capture input lineage or revisions moved"));
    check!(eq;page_count, 1);
    check!(eq;encoded.len(), 1);
    check!(eq;concurrency::authority_state(&store, &writer)?, authority);
    Ok(())
}

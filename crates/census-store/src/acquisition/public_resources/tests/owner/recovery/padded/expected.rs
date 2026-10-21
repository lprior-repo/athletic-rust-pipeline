use super::*;
use fjall::{Keyspace, Readable, Snapshot};

pub(super) fn admission(store: &Store, work: &AcquisitionWorkV1) -> TestResult<u64> {
    let snapshot = store.db.snapshot();
    let declaration = declaration(store)?;
    let mut one_pass = 0_u64;
    for key in [
        "schema:store_version",
        "schema:key_format",
        "schema:lifecycle_revision",
        "schema:created_by",
        "schema:created_at",
        CaptureProtocolVersion::V2.qualification_key(),
    ] {
        add(&mut one_pass, row(&snapshot, &store.meta, key.as_bytes())?)?;
    }
    add(
        &mut one_pass,
        row(
            &snapshot,
            &store.journal,
            &Store::journal_key("export-lineage-v1", "store-identity"),
        )?,
    )?;
    add(
        &mut one_pass,
        row(
            &snapshot,
            &store.journal,
            crate::acquisition::public_resources::DECLARATION.as_bytes(),
        )?,
    )?;
    add(
        &mut one_pass,
        row(&snapshot, &store.meta, crate::run::RUN_MANIFEST.as_bytes())?,
    )?;
    let key = crate::profiles::key(declaration.binding.selection.profile_id());
    add(&mut one_pass, row(&snapshot, &store.meta, key.as_bytes())?)?;
    add(
        &mut one_pass,
        row(
            &snapshot,
            &store.receipts,
            declaration.receipt()?.operation.as_bytes(),
        )?,
    )?;
    let id = crate::acquisition::public_resources::frontier::entry_id(&declaration, work)?;
    let mut total = one_pass
        .checked_mul(2)
        .ok_or("two public admission passes")?;
    add(
        &mut total,
        row(
            &snapshot,
            &store.journal,
            format!("{ENTRY_PREFIX}{id}").as_bytes(),
        )?,
    )?;
    add(
        &mut total,
        row(
            &snapshot,
            &store.receipts,
            format!("{FRONTIER_OPERATION_PREFIX}{id}").as_bytes(),
        )?,
    )?;
    Ok(total)
}

fn row(snapshot: &Snapshot, table: &Keyspace, key: &[u8]) -> TestResult<u64> {
    let raw = snapshot
        .get(table, key)?
        .ok_or("expected public admission row")?;
    Ok(u64::try_from(
        key.len()
            .checked_add(raw.len())
            .ok_or("public admission row bytes")?,
    )?)
}

fn add(total: &mut u64, bytes: u64) -> TestResult {
    *total = total.checked_add(bytes).ok_or("public admission sum")?;
    Ok(())
}

pub(super) fn lineage(store: &Store) -> TestResult<Vec<u8>> {
    let raw = store
        .journal
        .get(Store::journal_key("export-lineage-v1", "store-identity"))?
        .ok_or("original lineage row")?;
    Ok(raw.to_vec())
}

pub(super) fn body_path(store: &Store, input: &PreparedCaptureInputV1) -> std::path::PathBuf {
    store
        .http_cache_dir()
        .join("archive/bodies")
        .join(format!("{}.body", input.content_digest()))
}

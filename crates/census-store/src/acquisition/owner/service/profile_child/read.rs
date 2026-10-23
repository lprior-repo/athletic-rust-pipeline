use super::{refused, Entry, PREFIX};
use crate::acquisition::public_resources::{intake, read as public, PublicResourceDeclarationV1};
use crate::{AcquisitionReadBudget, Receipt, StoreResult};
use census_domain::model::{AcquisitionWorkV1, PublicSourceResourceV1};
use fjall::{Keyspace, Snapshot};

pub(in crate::acquisition) fn for_work(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &PublicResourceDeclarationV1,
    work: &AcquisitionWorkV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let entry = entry(
        snapshot,
        meta,
        journal,
        receipts,
        declaration,
        key(work)?.as_bytes(),
        budget,
    )?;
    if &entry.work != work {
        return Err(refused("public profile child work differs"));
    }
    Ok(())
}

pub(super) fn entry(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &PublicResourceDeclarationV1,
    key: &[u8],
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Entry> {
    let entry: Entry = public::record(snapshot, journal, key, budget)?
        .ok_or_else(|| refused("public profile child entry absent"))?;
    if entry.key().as_bytes() != key {
        return Err(refused("public profile child entry namespace"));
    }
    let receipt: Receipt =
        public::record(snapshot, receipts, entry.operation()?.as_bytes(), budget)?
            .ok_or_else(|| refused("public profile child receipt absent"))?;
    entry.require_receipt(declaration, &receipt)?;
    super::counter::read(snapshot, meta, declaration, &entry.work, budget)?
        .ok_or_else(|| refused("public profile child counter absent"))?;
    parent(
        snapshot,
        meta,
        journal,
        receipts,
        declaration,
        &entry,
        budget,
    )?;
    public::links(snapshot, journal, &entry.work, budget)?;
    Ok(entry)
}

pub(super) fn parent(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &PublicResourceDeclarationV1,
    entry: &Entry,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let packet = intake::startup::read_packet(snapshot, journal, receipts, entry.packet.as_str())?;
    entry.validate(declaration)?;
    entry.require_parent(&packet)?;
    intake::read_admission(snapshot, meta, journal, receipts, &packet, budget)?;
    Ok(())
}

pub(super) fn key(work: &AcquisitionWorkV1) -> StoreResult<String> {
    let athlete = work
        .slot()
        .public_resource()
        .and_then(PublicSourceResourceV1::athlete_id)
        .ok_or_else(|| refused("public profile child resource required"))?;
    Ok(format!(
        "{}{}/{}/{}",
        PREFIX,
        work.slot().owner_key(),
        work.slot().source_site().code(),
        athlete
    ))
}

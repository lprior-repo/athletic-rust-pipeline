use super::{refused, Entry, PublicResourceDeclarationV1};
use crate::acquisition::{facts::*, read as acquisition, AcquisitionReadBudget};
use crate::{Receipt, StoreResult};
use census_domain::model::*;
use fjall::{Keyspace, Snapshot};

pub(in crate::acquisition) fn for_work(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &PublicResourceDeclarationV1,
    work: &AcquisitionWorkV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let entry = validate_entry(
        snapshot,
        journal,
        receipts,
        super::key(work)?.as_bytes(),
        declaration,
        budget,
    )?;
    if &entry.work != work {
        return Err(refused("public roster child work differs"));
    }
    Ok(())
}

pub(in crate::acquisition) fn validate_entry(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    key: &[u8],
    declaration: &PublicResourceDeclarationV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Entry> {
    let entry: Entry = super::super::read::record(snapshot, journal, key, budget)?
        .ok_or_else(|| refused("public roster child entry absent"))?;
    if entry.key()?.as_bytes() != key {
        return Err(refused("public roster child namespace"));
    }
    let receipt: Receipt =
        super::super::read::record(snapshot, receipts, entry.operation()?.as_bytes(), budget)?
            .ok_or_else(|| refused("public roster child receipt absent"))?;
    entry.validate_receipt(declaration, &receipt)?;
    budget
        .public_parent(|budget| parent(snapshot, journal, receipts, declaration, &entry, budget))?;
    Ok(entry)
}

fn parent(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &PublicResourceDeclarationV1,
    entry: &Entry,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let expected = entry.parent_work(declaration)?;
    let id = expected.id().map_err(crate::CaptureError::Domain)?;
    if entry.parent.attempt().work_id() != &id
        || entry.parent.attempt().store_identity() != expected.slot().store_identity()
    {
        return Err(refused("public roster child parent identity"));
    }
    super::super::read::root_binding(snapshot, journal, receipts, declaration, &expected, budget)?;
    let facts = acquisition::read_work(
        snapshot,
        journal,
        receipts,
        &id,
        budget,
        CaptureProtocolVersion::V2,
    )?;
    if facts.work != expected
        || facts.result.as_ref().map(|value| &value.occurrence) != Some(&entry.parent)
    {
        return Err(refused("public roster child parent result"));
    }
    let attempt = facts
        .attempts
        .iter()
        .find(|attempt| &attempt.reservation.attempt == entry.parent.attempt())
        .ok_or_else(|| refused("public roster child parent attempt"))?;
    require_capture(entry, &facts.work, attempt)
}

fn require_capture(
    entry: &Entry,
    work: &AcquisitionWorkV1,
    attempt: &AttemptFacts,
) -> StoreResult<()> {
    let terminal = attempt
        .terminal
        .as_ref()
        .ok_or_else(|| refused("public roster child parent terminal"))?;
    if terminal.occurrence()? != entry.parent
        || !matches!(
            terminal.terminal(),
            AcquisitionTerminal::ResponseRetained {
                disposition: CaptureDispositionV1::ReusableResponse,
                ..
            }
        )
    {
        return Err(refused(
            "public roster child parent must retain original body",
        ));
    }
    let prepared = attempt
        .prepared
        .as_ref()
        .ok_or_else(|| refused("public roster child parent prepared"))?;
    let response = acquisition::last_response(attempt)?;
    if prepared.capture.content_digest() != &entry.body_digest
        || prepared.capture.content_bytes() != entry.body_bytes
        || prepared.capture.metadata_digest() != &entry.metadata_digest
        || response.status() != 200
        || response.observed_http().is_none()
    {
        return Err(refused("public roster child parent capture differs"));
    }
    crate::acquisition::backing::validate_metadata(work, attempt, &prepared.capture)
}

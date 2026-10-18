use super::super::*;
use super::{archive, encoding};
use crate::StoreSnapshot;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::value::RawValue;

mod originals;
pub(super) use originals::roots;

struct RetainedEntries {
    records: Vec<Vec<u8>>,
    receipts: Vec<Vec<u8>>,
}

pub(super) fn freeze(
    current: &CurrentResultSchoolCorrespondencesV1<'_, '_>,
    identity: &crate::FrozenPublicIdentityBundleV2,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<super::ResultSchoolSourceProofV1> {
    current.require_original_grant(grant)?;
    let view = current.snapshot();
    grant
        .charge_bytes(identity.bytes().len())
        .map_err(grant_error)?;
    let accepted = identity.replay()?;
    let head = view.attachment_public_head()?;
    if accepted.store_identity() != current.store_identity()
        || accepted.snapshot_sequence() != current.snapshot_sequence()
        || accepted.head() != &head
    {
        return Err(Error::SourceMismatch("frozen identity input binding").into());
    }
    let attachment_bundle = current.attachments.freeze_closure(grant)?;
    let proof = codec::encode(&current.ledger.proof, grant)?;
    let entries = retained_entries(view, &current.ledger, grant)?;
    let roots = roots(&current.ledger, grant)?;
    let mut external = Vec::new();
    external
        .try_reserve_exact(roots.len())
        .map_err(|_| Error::BudgetExceeded)?;
    roots.into_iter().try_for_each(|root| {
        if current
            .attachments
            .contains_retained_capture_root(&root, grant)?
        {
            return Ok(());
        }
        external.push(borrowed_prefix(root, &current.ledger.checked_roots, grant)?);
        Ok::<_, StoreError>(())
    })?;
    let archive = archive::Archive {
        schema: archive::SCHEMA.into(),
        store: current.store_identity().clone(),
        snapshot_sequence: current.snapshot_sequence(),
        inventory: current.inventory().clone(),
        enrollment: archive::raw(&proof)?,
        records: borrowed(&entries.records, grant)?,
        receipts: borrowed(&entries.receipts, grant)?,
        identity: archive::raw(identity.bytes())?,
        identity_digest: identity.digest().to_owned(),
        attachments: archive::raw(attachment_bundle.bytes())?,
        attachment_reference: attachment_bundle.reference().clone(),
        external,
    };
    let bytes = encoding::encode(&archive, grant)?;
    let reference = census_domain::evidence_subject_attachment::AttachmentObjectReferenceV1 {
        digest: codec::digest(&bytes, grant)?,
        bytes: u64::try_from(bytes.len()).map_err(|_| Error::BudgetExceeded)?,
    };
    Ok(super::ResultSchoolSourceProofV1 { bytes, reference })
}

fn retained_entries(
    view: &StoreSnapshot<'_>,
    ledger: &super::super::issuance::ledger::Ledger,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<RetainedEntries> {
    view.with_roster_source_snapshot_v1(|snapshot, _, journal, receipts| {
        let mut records = Vec::new();
        let mut held_receipts = Vec::new();
        records
            .try_reserve_exact(ledger.records.len())
            .map_err(|_| Error::BudgetExceeded)?;
        held_receipts
            .try_reserve_exact(ledger.records.len())
            .map_err(|_| Error::BudgetExceeded)?;
        ledger
            .records
            .iter()
            .enumerate()
            .try_for_each(|(ordinal, record)| {
                records.push(read::entry_value(
                    snapshot,
                    journal,
                    wire::record_key(ordinal)?.as_bytes(),
                    grant,
                )?);
                held_receipts.push(read::value(
                    snapshot,
                    receipts,
                    wire::issuance_operation(&record.command).as_bytes(),
                    grant,
                )?);
                Ok::<_, StoreError>(())
            })?;
        Ok(RetainedEntries {
            records,
            receipts: held_receipts,
        })
    })
}

fn borrowed<'a>(
    values: &'a [Vec<u8>],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<&'a RawValue>> {
    grant
        .charge_bytes(
            values
                .len()
                .checked_mul(std::mem::size_of::<&RawValue>())
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(values.len())
        .map_err(|_| Error::BudgetExceeded)?;
    values.iter().try_for_each(|bytes| {
        output.push(archive::raw(bytes)?);
        Ok::<_, StoreError>(())
    })?;
    Ok(output)
}

fn borrowed_prefix<'a>(
    root: crate::RetainedCaptureRootV1,
    held: &'a [super::super::source::checked::Root],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<archive::Prefix<'a>> {
    grant.charge_steps(held.len()).map_err(grant_error)?;
    let prefix = held
        .iter()
        .find(|prefix| prefix.descriptor.schema == root.schema && prefix.descriptor.key == root.key)
        .ok_or(Error::Corrupt("checked source root absent"))?;
    if prefix.descriptor != root {
        return Err(Error::Corrupt("checked source root differs").into());
    }
    grant
        .charge_bytes(
            prefix
                .rows
                .len()
                .checked_mul(std::mem::size_of::<archive::Row<'_>>())
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    grant.charge_steps(prefix.rows.len()).map_err(grant_error)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(prefix.rows.len())
        .map_err(|_| Error::BudgetExceeded)?;
    for (key, raw) in &prefix.rows {
        let size = key
            .len()
            .checked_add(2)
            .and_then(|n| n.checked_div(3))
            .and_then(|n| n.checked_mul(4))
            .ok_or(Error::BudgetExceeded)?;
        grant.charge_bytes(size).map_err(grant_error)?;
        let mut encoded = String::new();
        encoded
            .try_reserve_exact(size)
            .map_err(|_| Error::BudgetExceeded)?;
        STANDARD.encode_string(key, &mut encoded);
        rows.push(archive::Row {
            key: encoded,
            value: archive::raw(raw.as_ref())?,
        });
    }
    Ok(archive::Prefix { root, rows })
}

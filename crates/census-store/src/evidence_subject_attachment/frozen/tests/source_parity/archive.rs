use super::*;
use crate::evidence_subject_attachment::frozen::archive::{
    Archive, Head, Inventory as ArchiveInventory, Kind, Prefix, Reference, Root, Row,
};
use serde_json::value::RawValue;

use crate::evidence_subject_attachment::RetainedCaptureRootV1;

type Rows = Vec<(Vec<u8>, Box<RawValue>)>;

pub(super) fn capture_rows(view: &crate::StoreSnapshot<'_>) -> TestResult<Bundle> {
    capture_roots(view, &[])
}

pub(super) fn capture_roots(
    view: &crate::StoreSnapshot<'_>,
    roots: &[RetainedCaptureRootV1],
) -> TestResult<Bundle> {
    let mut journals = Rows::new();
    RetainedJournalReadSource::scan_journal_namespace(
        view,
        namespace()?,
        budget(16),
        &mut journals,
        |_| Ok(()),
        copy_row,
    )?;
    let mut receipts = Rows::new();
    view.visit_receipt_prefix(PREFIX, budget(16), &mut receipts, |_| Ok(()), copy_row)?;
    let mut external = Vec::new();
    for root in roots {
        let mut rows = Rows::new();
        RetainedJournalReadSource::scan_journal_namespace(
            view,
            JournalNamespace::new(root.schema.phase(), &root.key)?,
            budget(16),
            &mut rows,
            |_| Ok(()),
            copy_row,
        )?;
        external.push(rows);
    }
    let mut prefixes = vec![
        prefix(Kind::Attachment, &journals)?,
        prefix(Kind::Receipts, &receipts)?,
    ];
    for (root, rows) in roots.iter().zip(&external) {
        let mut group = prefix(Kind::External, rows)?;
        group.root = Some(Root {
            schema: root.schema,
            key: &root.key,
            payload: Reference {
                digest: root.payload.digest.as_str(),
                bytes: root.payload.bytes,
            },
        });
        prefixes.push(group);
    }
    let identity = view.store_identity()?;
    let head = view.attachment_public_head()?;
    let empty = AttachmentInventoryV1::empty()?;
    let archive = Archive {
        schema: "retained-evidence-subject-attachment-closure/v1",
        store: identity.as_str(),
        snapshot_sequence: view.sequence(),
        public_head: Head {
            sequence: head.sequence,
            digest: head.digest.as_str(),
        },
        inventory: ArchiveInventory {
            version: empty.version,
            records: empty.records,
            digest: empty.digest.as_str(),
        },
        prefixes,
    };
    let bytes = serde_json::to_vec(&archive)?;
    let expected = reference(&bytes)?;
    Ok(Bundle::from_owned_bytes(
        bytes,
        expected,
        &mut AttachmentReplayGrantV1::new(),
    )?)
}

fn copy_row(rows: &mut Rows, key: &[u8], raw: &[u8]) -> StoreResult<()> {
    let value: Box<RawValue> =
        serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
            key: "synthetic parity archive row".into(),
            source,
        })?;
    if value.get().as_bytes() != raw {
        return Err(
            Error::Corrupt("synthetic parity helper refuses physical outer whitespace").into(),
        );
    }
    rows.push((key.to_vec(), value));
    Ok(())
}

fn prefix(kind: Kind, rows: &Rows) -> TestResult<Prefix<'_>> {
    let encoded_bytes = rows.iter().try_fold(0_usize, |total, (key, value)| {
        total
            .checked_add(key.len())
            .and_then(|total| total.checked_add(value.get().len()))
            .ok_or(StoreError::CounterOverflow)
    })?;
    Ok(Prefix {
        kind,
        root: None,
        physical_rows: u64::try_from(rows.len())?,
        encoded_bytes: u64::try_from(encoded_bytes)?,
        rows: rows
            .iter()
            .map(|(key, value)| Row {
                key: key.clone(),
                value,
            })
            .collect(),
    })
}

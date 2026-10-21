use super::*;
use crate::read::JournalNamespace;
use crate::ReadBudget;
use serde_json::value::RawValue;

#[derive(Clone, Copy)]
pub(super) enum Group<'a> {
    Attachment,
    Receipts,
    External(&'a RetainedCaptureRootV1),
}
pub(super) struct Summary<'a> {
    pub(super) group: Group<'a>,
    pub(super) rows: u64,
    pub(super) bytes: u64,
    pub(super) json_rows: usize,
    pub(super) digest: [u8; 32],
}
#[derive(Default)]
pub(super) struct Totals {
    rows: usize,
    bytes: usize,
}
struct State<'a, W> {
    grant: &'a mut AttachmentReplayGrantV1,
    totals: &'a mut Totals,
    writer: &'a mut W,
    receipt: bool,
    rows: u64,
    bytes: u64,
    digest: Sha256,
}

pub(super) fn scan<'r>(
    view: &StoreSnapshot<'_>,
    group: Group<'r>,
    totals: &mut Totals,
    writer: &mut impl Write,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Summary<'r>> {
    steps(grant, 2)?;
    let mut state = State {
        grant,
        totals,
        writer,
        receipt: matches!(group, Group::Receipts),
        rows: 0,
        bytes: 0,
        digest: Sha256::new(),
    };
    match group {
        Group::Receipts => {
            view.visit_receipt_prefix(
                OPERATION_PREFIX.as_bytes(),
                ReadBudget {
                    rows: 4096,
                    bytes: 4 * 1024 * 1024,
                    record_bytes: 2048,
                },
                &mut state,
                |state| steps(state.grant, 1),
                accept,
            )?;
        }
        Group::Attachment | Group::External(_) => {
            let (namespace, rows) = match group {
                Group::External(root) => {
                    (
                        JournalNamespace::new(root.schema.phase(), &root.key)?,
                        crate::evidence_subject_attachment::external::MAX_RETAINED_CAPTURE_PHYSICAL_ROWS,
                    )
                }
                _ => (JournalNamespace::whole_phase(PHASE)?, 8192),
            };
            RetainedJournalReadSource::scan_journal_namespace(
                view,
                namespace,
                ReadBudget {
                    rows,
                    bytes: 4 * 1024 * 1024,
                    record_bytes: crate::MAX_JOURNAL_VALUE_BYTES,
                },
                &mut state,
                |state| steps(state.grant, 4),
                accept,
            )?;
        }
    }
    Ok(Summary {
        group,
        rows: state.rows,
        bytes: state.bytes,
        json_rows: 0,
        digest: state.digest.finalize().into(),
    })
}

fn accept<W: Write>(state: &mut State<'_, W>, key: &[u8], bytes: &[u8]) -> StoreResult<()> {
    let length = admit_row(state, key, bytes)?;
    steps(state.grant, 1)?;
    let raw: &RawValue = serde_json::from_slice(bytes).map_err(|source| StoreError::Json {
        detail: "frozen physical row".into(),
        source,
    })?;
    steps(state.grant, 1)?;
    if raw.get().as_bytes() != bytes {
        return Err(Error::Corrupt("frozen physical row has outer whitespace").into());
    }
    steps(state.grant, 1)?;
    if state.rows != 0 {
        sink::write(state.writer, b",")?;
    }
    sink::json(state.writer, &archive::BorrowedRow { key, value: raw })?;
    steps(state.grant, 1)?;
    state.digest.update(
        u64::try_from(key.len())
            .map_err(|_| StoreError::CounterOverflow)?
            .to_be_bytes(),
    );
    state.digest.update(key);
    state.digest.update(
        u64::try_from(bytes.len())
            .map_err(|_| StoreError::CounterOverflow)?
            .to_be_bytes(),
    );
    state.digest.update(bytes);
    state.rows = state
        .rows
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    state.bytes = state
        .bytes
        .checked_add(u64::try_from(length).map_err(|_| StoreError::CounterOverflow)?)
        .ok_or(StoreError::CounterOverflow)?;
    Ok(())
}

fn admit_row<W>(state: &mut State<'_, W>, key: &[u8], bytes: &[u8]) -> StoreResult<usize> {
    steps(state.grant, 1)?;
    bound(
        "frozen archive key bytes",
        key.len(),
        if state.receipt { 512 } else { 4101 },
    )?;
    bound(
        "frozen archive value bytes",
        bytes.len(),
        if state.receipt {
            2048
        } else {
            crate::MAX_JOURNAL_VALUE_BYTES
        },
    )?;
    let length = key
        .len()
        .checked_add(bytes.len())
        .ok_or(StoreError::CounterOverflow)?;
    state.totals.rows = state
        .totals
        .rows
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    state.totals.bytes = state
        .totals
        .bytes
        .checked_add(length)
        .ok_or(StoreError::CounterOverflow)?;
    bound("frozen archive rows", state.totals.rows, MAX_ROWS)?;
    bound(
        "frozen archive physical bytes",
        state.totals.bytes,
        MAX_ARCHIVE_BYTES,
    )?;
    Ok(length)
}

use super::*;
use state::{parse, State};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header<'a> {
    #[serde(borrow)]
    schema: &'a str,
    #[serde(borrow)]
    store: &'a str,
    snapshot_sequence: u64,
    #[serde(borrow)]
    public_head: Head<'a>,
    #[serde(borrow)]
    inventory: Inventory<'a>,
    #[serde(borrow)]
    prefixes: &'a RawValue,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Group<'a> {
    kind: Kind,
    #[serde(borrow)]
    root: Option<Root<'a>>,
    physical_rows: u64,
    encoded_bytes: u64,
    #[serde(borrow)]
    rows: &'a RawValue,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRow<'a> {
    #[serde(borrow)]
    key: &'a str,
    #[serde(borrow)]
    value: &'a RawValue,
}

pub(in super::super) fn read<'a>(
    bytes: &'a [u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Archive<'a>> {
    steps(grant, 1)?;
    bound("frozen archive bytes", bytes.len(), MAX_ARCHIVE_BYTES)?;
    let header: Header<'_> = serde_json::from_slice(bytes).map_err(|source| StoreError::Json {
        detail: "frozen attachment archive header".into(),
        source,
    })?;
    validate::header(
        header.schema,
        header.store,
        &header.public_head,
        &header.inventory,
    )?;
    let mut state = State::new(grant);
    let prefixes = super::array::read(header.prefixes, &mut state, |state| state.prefix(), prefix)?;
    let archive = Archive {
        schema: header.schema,
        store: header.store,
        snapshot_sequence: header.snapshot_sequence,
        public_head: header.public_head,
        inventory: header.inventory,
        prefixes,
    };
    validate::archive(&archive, grant)?;
    super::equality::require(&archive, bytes, grant)?;
    Ok(archive)
}

fn prefix<'a>(raw: &'a RawValue, state: &mut State<'_>) -> StoreResult<Prefix<'a>> {
    let group: Group<'_> = parse(raw.get())?;
    validate::root(group.kind, group.root.as_ref())?;
    state.receipt = group.kind == Kind::Receipts;
    let rows = super::array::read(group.rows, state, |state| state.admit_row(), row)?;
    Ok(Prefix {
        kind: group.kind,
        root: group.root,
        physical_rows: group.physical_rows,
        encoded_bytes: group.encoded_bytes,
        rows,
    })
}

fn row<'a>(raw: &'a RawValue, state: &mut State<'_>) -> StoreResult<Row<'a>> {
    bound(
        "frozen archive encoded row",
        raw.get().len(),
        crate::MAX_JOURNAL_VALUE_BYTES + 8264,
    )?;
    let row: RawRow<'_> = parse(raw.get())?;
    bound(
        "frozen archive encoded key",
        row.key.len(),
        if state.receipt { 1024 } else { 8202 },
    )?;
    if row.key.is_empty() || !row.key.len().is_multiple_of(2) || !row.key.bytes().all(lower_hex) {
        return Err(Error::Corrupt("frozen archive key spelling").into());
    }
    bound(
        "frozen archive row value",
        row.value.get().len(),
        if state.receipt {
            2048
        } else {
            crate::MAX_JOURNAL_VALUE_BYTES
        },
    )?;
    state.row(row.key.len() / 2, row.value.get().len())?;
    steps(state.grant, 1)?;
    let mut key = Vec::new();
    key.try_reserve_exact(row.key.len() / 2)
        .map_err(|_| Error::Domain(EvidenceSubjectAttachmentError::Allocation))?;
    for pair in row.key.as_bytes().chunks_exact(2) {
        let high = pair.first().copied().ok_or(StoreError::CounterOverflow)?;
        let low = pair.get(1).copied().ok_or(StoreError::CounterOverflow)?;
        key.push((digit(high) << 4) | digit(low));
    }
    Ok(Row {
        key,
        value: row.value,
    })
}

fn digit(byte: u8) -> u8 {
    if byte <= b'9' {
        byte - b'0'
    } else {
        byte - b'a' + 10
    }
}
pub(super) fn lower_hex(byte: u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
}

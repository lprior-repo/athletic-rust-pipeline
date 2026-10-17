use super::*;
use sha2::{Digest, Sha256};

pub(super) fn budget(rows: u64) -> ReadBudget {
    ReadBudget {
        rows,
        bytes: 4 * 1024 * 1024,
        record_bytes: crate::MAX_JOURNAL_VALUE_BYTES,
    }
}

pub(super) fn namespace() -> StoreResult<JournalNamespace<'static>> {
    JournalNamespace::whole_phase(PHASE)
}

pub(super) fn reference(bytes: &[u8]) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(bytes)))?,
        bytes: u64::try_from(bytes.len())?,
    })
}

pub(super) fn put(store: &Store, key: &str, value: &[u8]) -> TestResult {
    store
        .journal
        .insert(Store::journal_key(PHASE, key), value)?;
    Ok(())
}

pub(super) fn envelope(key: &str, payload: &str) -> String {
    format!("{{\"key\":\"{key}\",\"at\":\"original-time\",\"payload\": {payload} }}")
}

pub(super) fn chunked(store: &Store, key: &str, bytes: &[u8]) -> TestResult {
    let chunks: Vec<_> = bytes.chunks(JOURNAL_CHUNK_RAW).collect();
    put(store, key, &manifest_value(key, chunks.len(), bytes.len())?)?;
    for (ordinal, raw) in chunks.iter().enumerate() {
        store.journal.insert(
            chunk_key(&Store::journal_key(PHASE, key), ordinal)?,
            chunk_value(key, ordinal, chunks.len(), raw)?,
        )?;
    }
    Ok(())
}

pub(super) fn raw_text(
    source: &impl RetainedJournalReadSource,
    key: &str,
    bytes: usize,
) -> TestResult<Option<(String, String)>> {
    Ok(
        read_raw(source, PHASE, key, JournalReadBudget::new(bytes)?)?
            .map(|value| (value.at().to_owned(), value.payload().get().to_owned())),
    )
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Inventory {
    pub rows: u64,
    pub bytes: u64,
    pub digest: [u8; 32],
    pub roots: u64,
    pub visits: Vec<(u64, String)>,
}

pub(super) fn inventory(source: &impl RetainedJournalReadSource) -> TestResult<Inventory> {
    let mut state = (0_u64, Vec::new());
    let result = scan_roots(
        source,
        namespace()?,
        budget(16),
        &mut state,
        admit,
        |state, key| {
            state.1.push((state.0, key.to_owned()));
            Ok(())
        },
    )?;
    Ok(Inventory {
        rows: result.contents().physical_rows(),
        bytes: result.contents().encoded_bytes(),
        digest: *result.contents().digest(),
        roots: result.logical_roots(),
        visits: state.1,
    })
}

pub(super) fn admit<T>(state: &mut (u64, T)) -> StoreResult<()> {
    state.0 = state.0.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    Ok(())
}

pub(super) fn descriptor<T>(result: StoreResult<T>, reason: &str) -> TestResult {
    check!(
        matches!(result, Err(StoreError::JournalDescriptor { phase, key, detail })
        if phase == PHASE && key.is_empty() && detail == reason)
    );
    Ok(())
}

pub(super) fn outside<T>(result: StoreResult<T>) -> TestResult {
    check!(matches!(
        result,
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "frozen request is outside captured closure"
        )))
    ));
    Ok(())
}

//! The store fixture: the seeded corpus, its verification, and the batch builders.
use super::*;

/// A temporary store, one verified document, one verified batch, and the workbook digest.
pub(super) struct Dataset {
    pub(super) store: ArtifactStore,
    pub(super) _directory: TempDir,
    pub(super) workbook: WorkbookDigest,
    pub(super) document: Vec<u8>,
    pub(super) document_digest: EvidenceDigest,
    pub(super) batch: Vec<SourceRecord>,
    pub(super) batch_key: SourceRowKey,
}

impl Dataset {
    pub(super) fn build() -> Result<Self> {
        let directory = tempfile::tempdir().context("creating the bench directory")?;
        let store = ArtifactStore::open(&directory.path().join("store"))
            .context("opening the root artifact store")?;
        let workbook =
            WorkbookDigest::parse(WORKBOOK_HEX).context("parsing the workbook digest")?;
        let seeded = seed(&store, &workbook)?;
        println!("metric=bench_seed_documents value={SEED_DOCUMENTS} unit=documents");
        println!("metric=bench_document_bytes value={DOCUMENT_BYTES} unit=bytes");
        println!("metric=bench_batch_records value={BATCH_RECORDS} unit=records");
        println!(
            "artifact store seeded and read back: {SEED_DOCUMENTS} documents, \
             {} source rows over {} batches",
            SEED_BATCHES * BATCH_RECORDS,
            SEED_BATCHES
        );
        Ok(Self {
            store,
            _directory: directory,
            workbook,
            document: seeded.document,
            document_digest: seeded.digest,
            batch: seeded.batch,
            batch_key: seeded.key,
        })
    }
}

/// The document and batch the benches read and re-publish, plus the total seeded row count.
pub(super) struct Seeded {
    pub(super) document: Vec<u8>,
    pub(super) digest: EvidenceDigest,
    pub(super) batch: Vec<SourceRecord>,
    pub(super) key: SourceRowKey,
}

/// Publish `SEED_DOCUMENTS` documents and `SEED_BATCHES` batches, then require the store to read
/// every one of them back exactly as written.
pub(super) fn seed(store: &ArtifactStore, workbook: &WorkbookDigest) -> Result<Seeded> {
    let mut seeded_document = None;
    for index in 0..SEED_DOCUMENTS {
        let payload = document_payload(index);
        let digest = store
            .put_bytes(&payload)
            .with_context(|| format!("publishing seed document {index}"))?;
        ensure!(
            digest.as_str() == sha256_hex(&payload),
            "seed document {index} digest is not the SHA-256 of its bytes"
        );
        ensure!(
            store
                .get_bytes(&digest)
                .context("reading back a seed document")?
                == payload,
            "seed document {index} did not read back byte-for-byte"
        );
        seeded_document = Some((payload, digest));
    }
    let (document, document_digest) = seeded_document.context("no seed document was written")?;

    let mut seeded_batch = None;
    for batch in 0..SEED_BATCHES {
        let records = batch_records(batch * BATCH_RECORDS, BATCH_RECORDS)?;
        store
            .put_source_batch(workbook, &records)
            .with_context(|| format!("publishing seed batch {batch}"))?;
        seeded_batch = Some(records);
    }
    let batch = seeded_batch.context("no seed batch was written")?;
    let key = SourceRowKey::parse(&first_record(batch.as_slice())?.source_key)
        .context("parsing the first seeded source row key")?;
    verify_rows(store, workbook, &batch, &key)?;
    verify_absent(store)?;
    Ok(Seeded {
        document,
        digest: document_digest,
        batch,
        key,
    })
}

/// Every seeded row must read back field-for-field, and one unwritten row must read `None`.
pub(super) fn verify_rows(
    store: &ArtifactStore,
    workbook: &WorkbookDigest,
    batch: &[SourceRecord],
    key: &SourceRowKey,
) -> Result<()> {
    for record in batch {
        let read = store
            .source_record(workbook, &SourceRowKey::parse(&record.source_key)?)
            .context("reading back a seeded source row")?
            .context("a seeded source row is absent")?;
        ensure!(
            read.source_key == record.source_key
                && read.sheet == record.sheet
                && read.excel_row == record.excel_row
                && read.fields == record.fields,
            "seeded source row {} did not read back as written",
            record.source_key
        );
    }
    let absent = SourceRowKey::parse(&format!("{SHEET}:{}", absent_row()))?;
    ensure!(
        store
            .source_record(workbook, &absent)
            .context("reading an unwritten source row")?
            .is_none(),
        "an unwritten source row read back as present"
    );
    ensure!(
        store
            .source_record(workbook, key)
            .context("reading the first seeded source row")?
            .is_some(),
        "the first seeded source row is absent"
    );
    Ok(())
}

/// An unwritten digest must fail as `MissingArtifact`, not as stored bytes.
pub(super) fn verify_absent(store: &ArtifactStore) -> Result<()> {
    let absent = EvidenceDigest::parse(&sha256_hex(b"never published by the bench"))
        .context("parsing the absent document digest")?;
    match store.get_bytes(&absent) {
        Err(StoreError::MissingArtifact) => Ok(()),
        Err(other) => Err(anyhow::anyhow!(
            "reading an unwritten document failed with the wrong error: {other}"
        )),
        Ok(_) => Err(anyhow::anyhow!(
            "reading an unwritten document returned bytes"
        )),
    }
}

/// `count` synthetic source rows starting at `base + FIRST_DATA_ROW`.
pub(super) fn batch_records(base: usize, count: usize) -> Result<Vec<SourceRecord>> {
    (0..count)
        .map(|slot| {
            let index = base.checked_add(slot).context("record index overflow")?;
            let excel_row = u32::try_from(index)
                .context("row conversion overflow")?
                .checked_add(FIRST_DATA_ROW)
                .context("row overflow")?;
            let fields = SOURCE_HEADERS
                .iter()
                .map(|header| ((*header).to_owned(), format!("bench-{index}")))
                .collect::<BTreeMap<_, _>>();
            Ok(SourceRecord {
                source_key: format!("{SHEET}:{excel_row}"),
                sheet: SHEET.to_owned(),
                excel_row,
                fields,
            })
        })
        .collect()
}

/// The first record of a batch, or an error for an empty one.
pub(super) fn first_record(records: &[SourceRecord]) -> Result<&SourceRecord> {
    records.first().context("the batch has no records")
}

/// A 64 KiB document whose first bytes embed its index, so distinct payloads are distinct bytes.
pub(super) fn document_payload(index: usize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(DOCUMENT_BYTES);
    payload.extend_from_slice(format!("bench-document-{index:08}").as_bytes());
    let fill = u8::try_from(index % 251).unwrap_or(0);
    payload.resize(DOCUMENT_BYTES, fill);
    payload
}

/// SHA-256 of `bytes` as lowercase hex: the digest the store must return for a publication.
pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// An Excel row no seed batch writes.
pub(super) fn absent_row() -> u32 {
    u32::try_from(SEED_BATCHES * BATCH_RECORDS)
        .unwrap_or(u32::MAX - 1)
        .saturating_add(FIRST_DATA_ROW)
        .saturating_add(1)
}

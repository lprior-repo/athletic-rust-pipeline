//! Committed measurement for the **root** artifact store (`src/store.rs`, `ArtifactStore`).
//!
//! ```text
//! cargo bench -p athletic-rust-pipeline --bench artifact_store
//! ```
//!
//! **Which store this is.** The root `ArtifactStore` is the Fjall-backed `documents` /
//! `source_index` pair that preserves source workbooks, retained documents and source rows; it is
//! *not* the census store (`crates/midwest-census/src/store/**`). The two are separate databases
//! with separate metrics — read no number here as census "store ingest obs/s".
//!
//! **What is measured.** The three public entry points, on a temporary store:
//!
//! * `put_bytes` — content-addressed document publication. `fresh_document` pays one
//!   `PersistMode::SyncAll` commit per call (the durability floor of the substrate);
//!   `duplicate_document` re-publishes bytes already stored, which must deduplicate instead of
//!   writing again.
//! * `put_source_batch` — atomic source-row publication (`MAX_BATCH_RECORDS` rows per call);
//!   `fresh_batch` writes a batch whose rows were never stored, `replayed_batch` re-publishes the
//!   identical batch and must be a no-op.
//! * `source_record` / `get_bytes` — the point-read paths, including digest verification.
//!
//! **Self-asserting counts.** `seed` refuses to hand the benches a store it has not read back:
//! every seeded document is re-read through `get_bytes` (byte-for-byte) and its digest is checked
//! against an independent SHA-256 of the payload, every seeded source row is re-read through
//! `source_record` and compared field by field, an unwritten row must read `None`, and an unwritten
//! digest must fail with `MissingArtifact`. Every payload embeds its own index, so distinct payloads
//! are distinct by construction; a rate is never reported for a store that quietly deduplicated or
//! lost the dataset.
//!
//! **Footprint.** The fresh-write groups commit one synced write per iteration, so a full default
//! run writes the honest byte volume for that path into a temporary directory that is removed when
//! the run ends. Bound it with `cargo bench ... -- --measurement-time 2` when disk is tight.

use anyhow::{ensure, Context, Result};
use athletic_rust_pipeline::{
    domain::identity::{EvidenceDigest, SourceRowKey, WorkbookDigest},
    model::{SourceRecord, SOURCE_HEADERS},
    store::{ArtifactStore, StoreError},
};
use criterion::{BatchSize, Criterion, Throughput};
use sha2::{Digest, Sha256};
use std::{cell::Cell, collections::BTreeMap};
use tempfile::TempDir;

/// Worksheet name the synthetic source rows belong to.
const SHEET: &str = "Prospects";
/// Rows per `put_source_batch` call (a quarter of `MAX_BATCH_RECORDS`).
const BATCH_RECORDS: usize = 512;
/// Bytes per document written to the store.
const DOCUMENT_BYTES: usize = 64 * 1024;
/// Documents seeded and read back before measurement.
const SEED_DOCUMENTS: usize = 32;
/// Batches seeded and read back before measurement.
const SEED_BATCHES: usize = 4;
/// The workbook digest every synthetic source row is indexed under.
const WORKBOOK_HEX: &str = "b1e1c4d9f0a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4c";
/// First Excel row carrying data; row 1 is the header row.
const FIRST_DATA_ROW: u32 = 2;

fn main() {
    let dataset = Dataset::build()
        .unwrap_or_else(|error| panic!("artifact_store dataset is not usable: {error:#}"));
    let mut criterion = Criterion::default().configure_from_args();
    bench_put_bytes(&mut criterion, &dataset);
    bench_put_source_batch(&mut criterion, &dataset);
    bench_reads(&mut criterion, &dataset);
    criterion.final_summary();
}

/// A temporary store, one verified document, one verified batch, and the workbook digest.
struct Dataset {
    store: ArtifactStore,
    _directory: TempDir,
    workbook: WorkbookDigest,
    document: Vec<u8>,
    document_digest: EvidenceDigest,
    batch: Vec<SourceRecord>,
    batch_key: SourceRowKey,
}

impl Dataset {
    fn build() -> Result<Self> {
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
struct Seeded {
    document: Vec<u8>,
    digest: EvidenceDigest,
    batch: Vec<SourceRecord>,
    key: SourceRowKey,
}

/// Publish `SEED_DOCUMENTS` documents and `SEED_BATCHES` batches, then require the store to read
/// every one of them back exactly as written.
fn seed(store: &ArtifactStore, workbook: &WorkbookDigest) -> Result<Seeded> {
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
fn verify_rows(
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
fn verify_absent(store: &ArtifactStore) -> Result<()> {
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

/// `put_bytes`: the durability floor (one fresh synced commit per call) and the dedupe path.
fn bench_put_bytes(criterion: &mut Criterion, dataset: &Dataset) {
    let mut group = criterion.benchmark_group("artifact_store/put_bytes");
    group.throughput(Throughput::Elements(1));
    group.bench_function("fresh_document", |bencher| {
        let written = Cell::new(SEED_DOCUMENTS);
        bencher.iter_batched(
            || {
                let index = written.get();
                written.set(index.saturating_add(1));
                document_payload(index)
            },
            |payload| {
                let digest = checked(dataset.store.put_bytes(&payload));
                std::hint::black_box(digest);
            },
            BatchSize::SmallInput,
        )
    });
    group.throughput(Throughput::Elements(1));
    group.bench_function("duplicate_document", |bencher| {
        bencher.iter(|| {
            let digest = checked(dataset.store.put_bytes(&dataset.document));
            std::hint::black_box(digest);
        })
    });
    group.finish();
}

/// `put_source_batch`: fresh atomic publication versus an identical re-publish.
fn bench_put_source_batch(criterion: &mut Criterion, dataset: &Dataset) {
    let mut group = criterion.benchmark_group("artifact_store/put_source_batch");
    group.throughput(Throughput::Elements(row_count(BATCH_RECORDS)));
    group.bench_function("fresh_batch", |bencher| {
        let published = Cell::new(SEED_BATCHES);
        bencher.iter_batched(
            || {
                let batch = published.get();
                published.set(batch.saturating_add(1));

                checked(batch_records(batch * BATCH_RECORDS, BATCH_RECORDS))
            },
            |records| {
                checked(dataset.store.put_source_batch(&dataset.workbook, &records));
            },
            BatchSize::SmallInput,
        )
    });
    group.throughput(Throughput::Elements(row_count(BATCH_RECORDS)));
    group.bench_function("replayed_batch", |bencher| {
        bencher.iter(|| {
            checked(
                dataset
                    .store
                    .put_source_batch(&dataset.workbook, &dataset.batch),
            );
        })
    });
    group.finish();
}

/// The read paths: a full document (with digest verification) and one source row by key.
fn bench_reads(criterion: &mut Criterion, dataset: &Dataset) {
    let mut group = criterion.benchmark_group("artifact_store/read");
    group.throughput(Throughput::Bytes(row_count(DOCUMENT_BYTES)));
    group.bench_function("document", |bencher| {
        bencher.iter(|| {
            let bytes = checked(dataset.store.get_bytes(&dataset.document_digest));
            std::hint::black_box(bytes);
        })
    });
    group.throughput(Throughput::Elements(1));
    group.bench_function("source_record", |bencher| {
        bencher.iter(|| {
            let record = checked(
                dataset
                    .store
                    .source_record(&dataset.workbook, &dataset.batch_key),
            );
            std::hint::black_box(record);
        })
    });
    group.finish();
}

/// `count` synthetic source rows starting at `base + FIRST_DATA_ROW`.
fn batch_records(base: usize, count: usize) -> Result<Vec<SourceRecord>> {
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
fn first_record(records: &[SourceRecord]) -> Result<&SourceRecord> {
    records.first().context("the batch has no records")
}

/// A 64 KiB document whose first bytes embed its index, so distinct payloads are distinct bytes.
fn document_payload(index: usize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(DOCUMENT_BYTES);
    payload.extend_from_slice(format!("bench-document-{index:08}").as_bytes());
    let fill = u8::try_from(index % 251).unwrap_or(0);
    payload.resize(DOCUMENT_BYTES, fill);
    payload
}

/// SHA-256 of `bytes` as lowercase hex: the digest the store must return for a publication.
fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// An Excel row no seed batch writes.
fn absent_row() -> u32 {
    u32::try_from(SEED_BATCHES * BATCH_RECORDS)
        .unwrap_or(u32::MAX - 1)
        .saturating_add(FIRST_DATA_ROW)
        .saturating_add(1)
}

/// A row count as the `u64` a criterion throughput declaration needs.
fn row_count(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}

/// Fail the measurement when the workload itself fails rather than reporting a rate for it.
fn checked<T, E>(result: std::result::Result<T, E>) -> T
where
    E: std::fmt::Display,
{
    result.unwrap_or_else(|error| panic!("artifact store workload failed: {error}"))
}

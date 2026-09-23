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

mod fixtures;

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

use self::fixtures::{Dataset, batch_records, document_payload};

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

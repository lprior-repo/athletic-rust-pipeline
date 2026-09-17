use super::support::{ready, run_export, Fixture};
use athletic_rust_pipeline::{
    store::ArtifactStore, workbook_ingest::visit_records, workbook_verify::verify_fields,
};
use criterion::{BatchSize, Criterion, Throughput};
use std::hint::black_box;
use tempfile::TempDir;

pub fn benchmark(c: &mut Criterion) {
    let fixture = ready(Fixture::new());
    benchmark_workbook(c, &fixture);
    benchmark_store(c, &fixture);
    benchmark_export(c, &fixture);
}

fn benchmark_workbook(c: &mut Criterion, fixture: &Fixture) {
    let mut group = c.benchmark_group("workbook");
    let extra_headers = vec!["Fixture Scenario".to_owned()];
    group.throughput(Throughput::Elements(ready(u64::try_from(
        fixture.records.len(),
    ))));
    group.bench_function("stream_visit_records", |b| {
        b.iter(|| {
            let mut count = 0_u64;
            ready(visit_records(&fixture.workbook, |_record| {
                count = count.saturating_add(1);
                Ok(())
            }));
            black_box(count)
        })
    });
    group.bench_function("verify_fields", |b| {
        b.iter(|| {
            black_box(ready(verify_fields(
                &fixture.workbook,
                &fixture.output,
                &fixture.digest,
                &extra_headers,
            )))
        })
    });
    group.finish();
}

fn benchmark_store(c: &mut Criterion, fixture: &Fixture) {
    let mut group = c.benchmark_group("store");
    group.throughput(Throughput::Elements(1));
    // Fixture::new prepopulates this store; the reuse path only validates existing keys.
    group.bench_function("artifact_put_bytes_idempotent_reuse", |b| {
        b.iter(|| {
            black_box(ready(
                fixture
                    .store
                    .put_bytes(b"synthetic retained profile document"),
            ))
        })
    });
    // Fresh setup is excluded; the real API requests PersistMode::SyncAll.
    // No separate fsync is added or claimed by this benchmark.
    group.bench_function("artifact_put_bytes_fresh_publication", |b| {
        b.iter_batched(
            fresh_store,
            |(directory, store)| {
                let digest = ready(store.put_bytes(b"synthetic fresh profile document"));
                black_box((directory, store, digest))
            },
            BatchSize::PerIteration,
        )
    });
    group.bench_function("artifact_get_bytes", |b| {
        b.iter(|| black_box(ready(fixture.store.get_bytes(&fixture.document_digest))))
    });
    group.throughput(Throughput::Elements(ready(u64::try_from(
        fixture.records.len(),
    ))));
    group.bench_function("source_batch_write_idempotent_reuse", |b| {
        b.iter(|| {
            ready(
                fixture
                    .store
                    .put_source_batch(&fixture.digest, &fixture.records),
            )
        })
    });
    // Fresh setup is excluded; the real API requests PersistMode::SyncAll.
    // No separate fsync is added or claimed by this benchmark.
    group.bench_function("source_batch_write_fresh_persistence", |b| {
        b.iter_batched(
            || {
                let (directory, store) = fresh_store();
                (
                    directory,
                    store,
                    fixture.digest.clone(),
                    fixture.records.clone(),
                )
            },
            |(directory, store, digest, records)| {
                ready(store.put_source_batch(&digest, &records));
                black_box((directory, store, digest, records))
            },
            BatchSize::PerIteration,
        )
    });
    group.throughput(Throughput::Elements(1));
    group.bench_function("source_record_read", |b| {
        b.iter(|| {
            black_box(ready(
                fixture
                    .store
                    .source_record(&fixture.digest, &fixture.source_key),
            ))
        })
    });
    group.bench_function("source_key_page_read", |b| {
        b.iter(|| {
            black_box(ready(fixture.store.source_key_page(
                &fixture.digest,
                "Alpha",
                0,
                64,
            )))
        })
    });
    group.finish();
}

fn benchmark_export(c: &mut Criterion, fixture: &Fixture) {
    let mut group = c.benchmark_group("export");
    group.throughput(Throughput::Elements(ready(u64::try_from(
        fixture.records.len(),
    ))));
    let extra_headers = vec!["Fixture Scenario".to_owned()];
    group.bench_function("write_and_finish", |b| {
        b.iter_batched(
            || {
                let directory = ready(TempDir::new());
                (
                    directory,
                    fixture.manifest.clone(),
                    fixture.export_rows.clone(),
                )
            },
            |(directory, manifest, rows)| {
                let destination = directory.path().join("export.xlsx");
                ready(run_export(&manifest, &rows, &destination, &extra_headers));
                black_box((directory, manifest, rows))
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn fresh_store() -> (TempDir, ArtifactStore) {
    let directory = ready(TempDir::new());
    let store = ready(ArtifactStore::open(&directory.path().join("artifacts")));
    (directory, store)
}

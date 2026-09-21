//! Committed measurement for the `midwest-census` hot paths: result-file parsing, school-label
//! resolution, and the store's read-time entity merge.
//!
//! ```text
//! cargo bench -p midwest-census --bench core
//! ```
//!
//! **What is measured.** Three groups, each on an entry point the census actually drives:
//!
//! * `census/parse` — every parser the result-file and association walks reach, replayed on a
//!   committed fixture: `hytek::lines_from_html` + `hytek::parse` on the Hy-Tek HTML release,
//!   `hytek::lines_from_text` + `hytek::parse` on the plain-text report, `raceday::parse` on the
//!   RaceDay finish list, `milesplit::parse_roster` on a graded roster page, and
//!   `plain_names::parse_nsaa_directory` on the NSAA directory export. `archive_classification`
//!   measures `wiaa_results::artifact_format` over every committed archive artifact — the dispatch
//!   each artifact pays before a parser is chosen. Elements are the rows a parse publishes (result
//!   rows, roster athletes, directory schools), or the artifacts classified.
//! * `census/school_index` — `census_domain::model::normalize_name` over the synthetic label corpus,
//!   `SchoolIndex::resolve` over those labels against a synthetic school snapshot, and
//!   `SchoolIndex::from_schools` over the snapshot. Elements are labels, or schools for the build.
//! * `census/merge` — the read half of the store, `Store::scan`: a synthetic observation batch
//!   appended once to a temporary store, then scanned per iteration (key walk + decode +
//!   `Entity::merge` + `Entity::publish`). Elements are the observations folded.
//!
//! **Corpus.** Fixture bodies are read from `tests/fixtures/**` once, before any measurement. The
//! school-index and merge corpora are synthetic, built from the literal tables in `core/labels.rs`
//! and `core/merge.rs` plus the seeded LCG in `core/lcg.rs`; nothing reads entropy, the clock or the
//! network, so a run is reproducible from those files alone. Every corpus is built once, outside the
//! timed region, and its length is the throughput element count, so each reported rate is
//! elements/s of the measured operation.
//!
//! **Self-asserting counts.** No rate is reported for a corpus that lost its shape. `Corpus::build`
//! in each module refuses to hand over a corpus until: every fixture parses to a non-empty published
//! row set and the archive classifies to something; every synthetic label resolves to the school and
//! match kind its recipe was built for, with the decoy labels still unresolved and all four outcomes
//! (exact, abbreviation, partial, unresolved) present; and the merge batch folds to exactly the
//! seeded distinct entity count, with first-writer fields surviving later observations, the alias
//! union intact, the longer name kept while the minted name does not move, and half the coach
//! mailboxes withheld by the contact policy.
//!
//! **Footprint.** The merge group seeds a Fjall store in a temporary directory (1_792 small rows)
//! and the directory is removed when the run ends; measurement itself is read-only. `cargo bench
//! -p midwest-census --bench core -- --warm-up-time 1 --measurement-time 2` bounds a full run when
//! the default criterion schedule is too slow for the machine.

// Helpers live in `benches/core/`; a bench target's root file resolves `mod` names next to itself,
// so the paths are explicit (the same shape the integration-test roots under `tests/` use).
#[path = "core/fixtures.rs"]
mod fixtures;
#[path = "core/labels.rs"]
mod labels;
#[path = "core/lcg.rs"]
mod lcg;
#[path = "core/merge.rs"]
mod merge;

use census_domain::model::normalize_name;
use criterion::{Criterion, Throughput};
use midwest_census::school_index::SchoolIndex;

/// Group ids. Stable, and unique per bench: a reported id is `<group>/<bench>`.
const PARSE_GROUP: &str = "census/parse";
const INDEX_GROUP: &str = "census/school_index";
const MERGE_GROUP: &str = "census/merge";

fn main() {
    let parse = fixtures::Corpus::build()
        .unwrap_or_else(|error| panic!("the parse corpus is not usable: {error:#}"));
    let labels = labels::Corpus::build()
        .unwrap_or_else(|error| panic!("the school-index corpus is not usable: {error:#}"));
    let batch = merge::Dataset::build()
        .unwrap_or_else(|error| panic!("the merge batch is not usable: {error:#}"));

    let mut criterion = Criterion::default().configure_from_args();
    bench_parse(&mut criterion, &parse);
    bench_school_index(&mut criterion, &labels);
    bench_merge(&mut criterion, &batch);
    criterion.final_summary();
}

/// `census/parse`: every committed fixture through the parser that owns its format, then the
/// classification every artifact pays before a parser is chosen.
fn bench_parse(criterion: &mut Criterion, corpus: &fixtures::Corpus) {
    let mut group = criterion.benchmark_group(PARSE_GROUP);
    for case in corpus.cases() {
        group.throughput(Throughput::Elements(elements(case.rows())));
        group.bench_function(case.id(), |bencher| {
            bencher.iter(|| {
                let rows = case.parse().unwrap_or_else(|error| {
                    panic!("the {} fixture failed to parse: {error:#}", case.file())
                });
                std::hint::black_box(rows);
            })
        });
    }
    group.throughput(Throughput::Elements(elements(corpus.artifacts())));
    group.bench_function("archive_classification", |bencher| {
        bencher.iter(|| {
            let claimed = corpus.classify_artifacts();
            std::hint::black_box(claimed);
        })
    });
    group.finish();
}

/// `census/school_index`: normalization, resolution and index construction over the label corpus.
fn bench_school_index(criterion: &mut Criterion, corpus: &labels::Corpus) {
    let labels = corpus.cases().len();
    let mut group = criterion.benchmark_group(INDEX_GROUP);
    group.throughput(Throughput::Elements(elements(labels)));
    group.bench_function("normalize_label", |bencher| {
        bencher.iter(|| {
            let width: usize = corpus
                .cases()
                .iter()
                .map(|case| normalize_name(&case.label).len())
                .sum();
            std::hint::black_box(width);
        })
    });
    group.throughput(Throughput::Elements(elements(labels)));
    group.bench_function("resolve_label", |bencher| {
        bencher.iter(|| {
            let resolved = corpus
                .cases()
                .iter()
                .filter(|case| corpus.index().resolve(case.state, &case.label).is_some())
                .count();
            std::hint::black_box(resolved);
        })
    });
    group.throughput(Throughput::Elements(elements(corpus.schools().len())));
    group.bench_function("build_index", |bencher| {
        bencher.iter(|| {
            let index = SchoolIndex::from_schools(corpus.schools());
            std::hint::black_box(index.school_count());
        })
    });
    group.finish();
}

/// `census/merge`: `Store::scan` over the synthetic observation batch, per table.
fn bench_merge(criterion: &mut Criterion, dataset: &merge::Dataset) {
    let mut group = criterion.benchmark_group(MERGE_GROUP);
    group.throughput(Throughput::Elements(elements(
        dataset.school_observations(),
    )));
    group.bench_function("schools_scan", |bencher| {
        bencher.iter(|| {
            let rows = dataset
                .scan_schools()
                .unwrap_or_else(|error| panic!("scanning the school batch failed: {error:#}"));
            std::hint::black_box(rows.len());
        })
    });
    group.throughput(Throughput::Elements(elements(dataset.coach_observations())));
    group.bench_function("coaches_scan", |bencher| {
        bencher.iter(|| {
            let rows = dataset
                .scan_coaches()
                .unwrap_or_else(|error| panic!("scanning the coach batch failed: {error:#}"));
            std::hint::black_box(rows.len());
        })
    });
    group.finish();
}

/// An element count as the `u64` a criterion throughput declaration needs.
fn elements(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}

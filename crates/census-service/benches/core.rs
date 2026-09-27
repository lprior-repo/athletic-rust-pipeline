
#[path = "core/fixtures.rs"]
mod fixtures;
#[path = "core/labels.rs"]
mod labels;
#[path = "core/lcg.rs"]
mod lcg;
#[path = "core/merge.rs"]
mod merge;

use census_domain::model::normalize_name;
use census_domain::school_index::SchoolIndex;
use criterion::{Criterion, Throughput};

const PARSE_GROUP: &str = "census/parse";
const INDEX_GROUP: &str = "census/school_index";
const MERGE_GROUP: &str = "census/merge";

fn main() {
    let parse = fixtures::Corpus::build()
        .unwrap_or_else(|error| refuse(format!("the parse corpus is not usable: {error:#}")));
    let labels = labels::Corpus::build().unwrap_or_else(|error| {
        refuse(format!("the school-index corpus is not usable: {error:#}"))
    });
    let batch = merge::Dataset::build()
        .unwrap_or_else(|error| refuse(format!("the merge batch is not usable: {error:#}")));

    let mut criterion = Criterion::default().configure_from_args();
    bench_parse(&mut criterion, &parse);
    bench_school_index(&mut criterion, &labels);
    bench_merge(&mut criterion, &batch);
    criterion.final_summary();
}

fn bench_parse(criterion: &mut Criterion, corpus: &fixtures::Corpus) {
    let mut group = criterion.benchmark_group(PARSE_GROUP);
    for case in corpus.cases() {
        group.throughput(Throughput::Elements(elements(case.rows())));
        group.bench_function(case.id(), |bencher| {
            bencher.iter(|| {
                let rows = case.parse().unwrap_or_else(|error| {
                    refuse(format!(
                        "the {} fixture failed to parse: {error:#}",
                        case.file()
                    ))
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

fn bench_merge(criterion: &mut Criterion, dataset: &merge::Dataset) {
    let mut group = criterion.benchmark_group(MERGE_GROUP);
    group.throughput(Throughput::Elements(elements(
        dataset.school_observations(),
    )));
    group.bench_function("schools_scan", |bencher| {
        bencher.iter(|| {
            let rows = dataset.scan_schools().unwrap_or_else(|error| {
                refuse(format!("scanning the school batch failed: {error:#}"))
            });
            std::hint::black_box(rows.len());
        })
    });
    group.throughput(Throughput::Elements(elements(dataset.coach_observations())));
    group.bench_function("coaches_scan", |bencher| {
        bencher.iter(|| {
            let rows = dataset.scan_coaches().unwrap_or_else(|error| {
                refuse(format!("scanning the coach batch failed: {error:#}"))
            });
            std::hint::black_box(rows.len());
        })
    });
    group.finish();
}

fn refuse(message: String) -> ! {
    eprintln!("{message}");
    std::process::exit(2)
}

fn elements(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}

//! National-scale measurement of the three stages a census run pays per unit of work: parsing one
//! captured result file, resolving the school labels a result file publishes, and reading the
//! performance observations a run wrote back out of the store.
//!
//! ```text
//! cargo bench -p midwest-census --bench pipeline
//! ```
//!
//! Bodies come from `tests/fixtures/**`; everything else is synthetic and deterministic, built from
//! the constants here — no entropy, the clock or the network — so a run is reproducible from this
//! file alone. No rate is reported for a corpus that lost its shape: each group refuses to measure
//! unless its recipe's claims hold. `-- --warm-up-time 1 --measurement-time 2` bounds a full run.

mod fixtures;

use anyhow::{ensure, Context, Result};
use census_crawl::wiaa_results::{artifact_format, parse_result_body, ArtifactFormat};
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, EventKind, Evidence, Gender, GradYear, Grade, Mark, SchoolId,
    SchoolYear, SourceRef, Sport, TimingMethod,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use criterion::{Criterion, Throughput};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use self::fixtures::{fixture, performance_observations};

/// Group ids. Stable, and unique per bench: a reported id is `<group>/<bench>`.
const RESULT_FILE_GROUP: &str = "pipeline/result_file";
const LABEL_GROUP: &str = "pipeline/school_labels";
const MERGE_GROUP: &str = "pipeline/merge";

/// The committed artifact the parse group replays: the largest Hy-Tek HTML release in the corpus
/// (32 parsed rows, 0 skipped, per the committed golden), read through the classifier and dispatch
/// `wiaa_results::run` uses, stamped `ARCHIVE_YEAR`.
const RESULT_FILE: &str = "d1boysstateresults-dash.htm";
const RESULT_DIR: &str = "wiaa_results";
const ARCHIVE_YEAR: i16 = 2025;

/// The synthetic national snapshot: 100 schools in every jurisdiction, each published under its
/// canonical and uppercase spelling, and every fourth one also as its relay squad (`<school> A`).
const SCHOOLS_PER_JURISDICTION: usize = 100;
const SCHOOL_PREFIX: &str = "Benchmark Academy";
const SQUAD_LABEL_EVERY: usize = 4;

/// The synthetic store batch: 5_000 performances observed four times each, over one championship
/// meet per jurisdiction. The first observation of each carries the fields the read-time merge has
/// to keep; the later ones contradict every one of them.
const PERFORMANCES: usize = 5_000;
const OBSERVATIONS_PER_PERFORMANCE: usize = 4;
const MEET_NAME: &str = "Benchmark State Championships";
const MEET_DATE: &str = "2025-06-07";

fn main() {
    or_fatal(run());
}

/// Measure every group; a corpus that lost its shape stops the run before any rate is reported.
fn run() -> Result<()> {
    let mut criterion = Criterion::default().configure_from_args();
    bench_result_file(&mut criterion)?;
    bench_school_labels(&mut criterion)?;
    bench_merge(&mut criterion)?;
    criterion.final_summary();
    Ok(())
}

/// The value, or the benchmark stops with the reason on stderr.
///
/// A bench has no caller to return to, and a number produced from a corpus that lost its shape is
/// worse than no number, so a failed step reports and exits instead of unwrapping.
fn or_fatal<T, E: std::fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => {
            eprintln!("the pipeline benchmark could not run: {error:#}");
            std::process::exit(1);
        }
    }
}

/// `pipeline/result_file`: what the archive run pays per captured artifact — the format decision
/// the runner makes before a parser is chosen, then the parse into the `result_file` shape the
/// mapper consumes. Elements are the result rows the parse publishes, so the rate is rows/s of the
/// stage that turns the largest captured release into a meet.
fn bench_result_file(criterion: &mut Criterion) -> Result<()> {
    let body = fixture(RESULT_DIR, RESULT_FILE)?;
    let format = artifact_format("htm", Some(&body));
    let parse = |body: &str| {
        let selected = artifact_format("htm", Some(body));
        parse_result_body(
            body.as_bytes(),
            selected,
            SourceRef::new(RESULT_DIR, None),
            ARCHIVE_YEAR,
        )
    };
    let meet = parse(&body).with_context(|| format!("{RESULT_FILE} parsed to no meet"))?;
    let parsed = format == ArtifactFormat::HytekHtml && meet.rows_parsed > 0;
    ensure!(parsed, "{RESULT_FILE} is not a parsed Hy-Tek release");
    let rows = u64::try_from(meet.rows_parsed).unwrap_or(u64::MAX);

    let mut group = criterion.benchmark_group(RESULT_FILE_GROUP);
    group.throughput(Throughput::Elements(rows));
    group.bench_function("parse", |bencher| {
        bencher.iter(|| {
            std::hint::black_box(parse(&body).map(|meet| meet.rows_parsed));
        })
    });
    group.finish();
    Ok(())
}

/// `pipeline/school_labels`: what the read-time resolver costs over a national school snapshot —
/// `SchoolIndex::from_schools` over every school, then `SchoolIndex::resolve` over the labels one
/// result file publishes, plus one label per jurisdiction taken from a school of another
/// jurisdiction, which must stay unresolved.
///
/// Sizes: 100 schools in each of the 49 jurisdictions (CENSUS_SCOPE) keeps every state's
/// bucket equal, so no jurisdiction dominates the lookup mix, and 49 × 100 is the order of
/// a national snapshot.
/// Elements are the labels, so the rate is label resolutions per second.
fn bench_school_labels(criterion: &mut Criterion) -> Result<()> {
    let mut schools = Vec::new();
    let mut labels: Vec<(UsJurisdiction, String, Option<SchoolId>)> = Vec::new();
    // 49-state product scope (CENSUS_SCOPE): the label resolver bench measures throughput over
    // the jurisdictions the census run actually covers.
    for jurisdiction in UsJurisdiction::CENSUS_SCOPE {
        for slot in 0..SCHOOLS_PER_JURISDICTION {
            let name = format!("{SCHOOL_PREFIX} {} {slot:03}", jurisdiction.code());
            let expected = CanonicalSchool::mint(jurisdiction, &name, &normalize_name(&name));
            schools.push(CanonicalSchool::new(jurisdiction, &name, normalize_name(&name)).0);
            labels.push((jurisdiction, name.clone(), Some(expected.clone())));
            labels.push((jurisdiction, name.to_uppercase(), Some(expected.clone())));
            if slot % SQUAD_LABEL_EVERY == 0 {
                labels.push((jurisdiction, format!("{name} A"), Some(expected)));
            }
        }
    }
    // A school of the next jurisdiction, published here: the resolver keys per jurisdiction, so
    // every one of these must stay unresolved.
    // 49-state product scope (CENSUS_SCOPE): cross-jurisdiction labels for product scope.
    let all = UsJurisdiction::CENSUS_SCOPE;
    for (jurisdiction, next) in all.iter().zip(all.iter().cycle().skip(1)) {
        labels.push((
            *jurisdiction,
            format!("{SCHOOL_PREFIX} {} 000", next.code()),
            None,
        ));
    }
    let index = SchoolIndex::from_schools(&schools);
    let snapshot_holds = index.school_count() == schools.len();
    ensure!(snapshot_holds, "the index lost snapshot schools");
    for (jurisdiction, label, expected) in &labels {
        let observed = index.resolve(*jurisdiction, label).map(|(id, _kind)| id);
        ensure!(
            observed.as_ref() == expected.as_ref(),
            "label {label:?} in {} resolved against its recipe",
            jurisdiction.code()
        );
    }

    let mut group = criterion.benchmark_group(LABEL_GROUP);
    group.throughput(Throughput::Elements(
        u64::try_from(labels.len()).unwrap_or(u64::MAX),
    ));
    group.bench_function("resolve", |bencher| {
        bencher.iter(|| {
            let resolved = labels
                .iter()
                .filter(|(jurisdiction, label, _)| index.resolve(*jurisdiction, label).is_some())
                .count();
            std::hint::black_box(resolved);
        })
    });
    group.finish();
    Ok(())
}

/// `pipeline/merge`: what reading the performance table back costs at national volume — the key
/// walk, the JSON decode, `Entity::merge` over four observations per performance, `Entity::publish`
/// and, for `consolidate`, the materialized JSONL snapshot plus the store flush.
///
/// Sizes: 20_000 appended observations folding to 5_000 performances is the read the report, the
/// workbook and the bests reducer each pay over one session's results. Elements are the appended
/// observations, so `consolidate` carries the snapshot write and its `SyncAll` on top.
fn bench_merge(criterion: &mut Criterion) -> Result<()> {
    let dir = tempfile::tempdir().context("creating the temporary store directory")?;
    let store = Store::open(dir.path()).context("opening the temporary census store")?;
    let batch = performance_observations()?;
    store
        .append_many(Table::Performances, &batch)
        .context("appending the performance batch")?;
    let observations = batch.len();

    let rows = store
        .scan::<CanonicalPerformance>(Table::Performances)
        .context("scanning the batch once to verify its fold")?;
    let meets: BTreeSet<&str> = rows.iter().map(|row| row.meet.as_str()).collect();
    // 49-state product scope (CENSUS_SCOPE): matches the meets count from CENSUS_SCOPE fixture batch.
    let fold_holds =
        rows.len() == PERFORMANCES && meets.len() == UsJurisdiction::CENSUS_SCOPE.len();
    ensure!(fold_holds, "the batch lost meets or performances");
    let kept = rows
        .iter()
        .filter(|row| {
            row.place == Some(1)
                && row.wind_mps == Some(1.2)
                && row.timing == Some(TimingMethod::Fat)
                && row.observed_grade == Grade::new(11)
                && row.evidence.len() == OBSERVATIONS_PER_PERFORMANCE
        })
        .count();
    ensure!(
        kept == PERFORMANCES,
        "{kept} lost the first writer's fields"
    );

    let mut group = criterion.benchmark_group(MERGE_GROUP);
    group.throughput(Throughput::Elements(
        u64::try_from(observations).unwrap_or(u64::MAX),
    ));
    group.bench_function("scan", |bencher| {
        bencher.iter(|| {
            let rows = or_fatal(store.scan::<CanonicalPerformance>(Table::Performances));
            std::hint::black_box(rows.len());
        })
    });
    group.bench_function("consolidate", |bencher| {
        bencher.iter(|| {
            let snapshot = or_fatal(store.consolidate::<CanonicalPerformance>(
                Table::Performances,
                &store.out_dir().join("performances.jsonl"),
            ));
            std::hint::black_box((snapshot.rows, snapshot.withheld));
        })
    });
    group.finish();
    Ok(())
}

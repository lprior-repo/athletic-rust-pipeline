#[path = "pipeline/fixtures.rs"]
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

const RESULT_FILE_GROUP: &str = "pipeline/result_file";
const LABEL_GROUP: &str = "pipeline/school_labels";
const MERGE_GROUP: &str = "pipeline/merge";

const RESULT_FILE: &str = "d1boysstateresults-dash.htm";
const RESULT_DIR: &str = "wiaa_results";
const ARCHIVE_YEAR: i16 = 2025;

const SCHOOLS_PER_JURISDICTION: usize = 100;
const SCHOOL_PREFIX: &str = "Benchmark Academy";
const SQUAD_LABEL_EVERY: usize = 4;

const PERFORMANCES: usize = 5_000;
const OBSERVATIONS_PER_PERFORMANCE: usize = 4;
const MEET_NAME: &str = "Benchmark State Championships";
const MEET_DATE: &str = "2025-06-07";

fn main() {
    or_fatal(run());
}

fn run() -> Result<()> {
    let mut criterion = Criterion::default().configure_from_args();
    bench_result_file(&mut criterion)?;
    bench_school_labels(&mut criterion)?;
    bench_merge(&mut criterion)?;
    criterion.final_summary();
    Ok(())
}

fn or_fatal<T, E: std::fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => {
            eprintln!("the pipeline benchmark could not run: {error:#}");
            std::process::exit(1);
        }
    }
}

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
    let rows = u64::try_from(meet.rows_parsed).map_or(u64::MAX, |value| value);

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

fn bench_school_labels(criterion: &mut Criterion) -> Result<()> {
    let mut schools = Vec::new();
    let mut labels: Vec<(UsJurisdiction, String, Option<SchoolId>)> = Vec::new();
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
        u64::try_from(labels.len()).map_or(u64::MAX, |value| value),
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
        u64::try_from(observations).map_or(u64::MAX, |value| value),
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
            std::hint::black_box(snapshot.rows);
        })
    });
    group.finish();
    Ok(())
}

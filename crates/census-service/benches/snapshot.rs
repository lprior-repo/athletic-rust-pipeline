use anyhow::{ensure, Context, Result};
use census_domain::model::{
    CanonicalAthlete, Confidence, Gender, GradYear, Grade, ObservedGrade, SchoolId, SchoolYear,
    SourceAthleteObservation, SourceIdentity, SourceNamespace, SourceObservation, SourceRef,
};
use census_store::{Store, StoreBatch, Table};
use criterion::{Criterion, Throughput};

const SUBJECTS: u32 = 20_000;

fn main() -> Result<()> {
    let (_directory, store) = dataset()?;
    validate(&store)?;
    let snapshot = store.snapshot();
    let mut criterion = Criterion::default().configure_from_args();
    let mut group = criterion.benchmark_group("pipeline/snapshot");
    group.throughput(Throughput::Elements(u64::from(SUBJECTS)));
    group.bench_function("athlete_evidence", |bencher| {
        bencher.iter(|| {
            let rows = match snapshot.athletes() {
                Ok(value) => value,
                Err(error) => refuse(format!("snapshot read failed: {error}")),
            };
            std::hint::black_box(rows);
        });
    });
    group.finish();
    criterion.final_summary();
    Ok(())
}

fn dataset() -> Result<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let school = SchoolId::mint("sch", &["snapshot-benchmark"]);
    let supported = ObservedGrade {
        grade: Grade::new(12).context("grade")?,
        school_year: SchoolYear::new(2026).context("school year")?,
        source: SourceRef::new("benchmark", None),
    };
    let unsupported = ObservedGrade {
        school_year: SchoolYear::new(2040).context("school year")?,
        ..supported.clone()
    };
    let mut batch = store.write_batch();
    for ordinal in 0..SUBJECTS {
        append_subject(&mut batch, &school, ordinal, &supported, &unsupported)?;
    }
    batch.commit()?;
    Ok((directory, store))
}

fn append_subject(
    batch: &mut StoreBatch<'_>,
    school: &SchoolId,
    ordinal: u32,
    supported: &ObservedGrade,
    unsupported: &ObservedGrade,
) -> Result<()> {
    let provider_id = ordinal.to_string();
    let name = format!("Benchmark Subject {ordinal}");
    let mut athlete = CanonicalAthlete::new(
        school,
        &name,
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, &provider_id),
    );
    athlete.observed_grades.push(supported.clone());
    batch.append_many(Table::Athletes, std::slice::from_ref(&athlete))?;
    let raw = SourceObservation::Athlete(
        SourceAthleteObservation::new(
            SourceNamespace::MilesplitAthlete,
            provider_id,
            format!("profile:{ordinal}"),
            name,
            "2026-09-30",
        )
        .with_grade(Some(unsupported.clone())),
    );
    batch.append_many(Table::SourceObservations, std::slice::from_ref(&raw))?;
    Ok(())
}

fn validate(store: &Store) -> Result<()> {
    let rows = store.snapshot().athletes()?;
    ensure!(
        u32::try_from(rows.len())? == SUBJECTS,
        "subject count changed"
    );
    ensure!(
        rows.iter().all(|row| {
            row.grad_year == GradYear::CO2027
                && row.derived_cohort_confidence() == Some(Confidence::LOW)
                && row.observed_grades.len() == 2
                && row
                    .observed_grades
                    .iter()
                    .any(|grade| grade.grad_year().is_none())
                && row
                    .observed_grades
                    .iter()
                    .any(|grade| grade.grad_year() == Some(GradYear::CO2027))
        }),
        "snapshot lost supported or unsupported cohort evidence"
    );
    Ok(())
}

fn refuse(message: String) -> ! {
    eprintln!("{message}");
    std::process::exit(2)
}

use super::*;
use census_domain::model::AppliedAthleteIdentity;

type RetainedState = (u64, Vec<(Table, census_store::TableWalk)>, String);

pub(super) fn retained_state(root: &Path) -> TestResult<RetainedState> {
    let store = Store::open(root)?;
    let tables = [
        Table::AthleteIdentityDecisions,
        Table::SourceIdentities,
        Table::ReviewCases,
        Table::Conflicts,
        Table::Coverage,
        Table::Snapshots,
    ];
    let rows = tables
        .into_iter()
        .map(|table| store.walk_table(table).map(|rows| (table, rows)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        store.derived_generation(),
        rows,
        store.snapshot().tables_digest(&Table::ALL)?,
    ))
}

pub(super) fn check_managed_only(root: &Path, output: &std::process::Output) -> TestResult {
    let stdout = std::str::from_utf8(&output.stdout)?;
    check!(
        !stdout
            .lines()
            .any(|line| ["index\t", "consolidate\t", "report\t"]
                .iter()
                .any(|prefix| line.starts_with(prefix))),
        "{stdout}"
    );
    for name in [
        "athletes.jsonl",
        "schools.jsonl",
        "census-all-sources.json",
        "census-core.json",
        "census-by-state.csv",
        "census-by-state-core.csv",
        "best-results-co2027.csv",
    ] {
        check!(
            !root.join("out").join(name).try_exists()?,
            "flat artifact was written: {name}"
        );
    }
    Ok(())
}

#[test]
fn failed_gather_cannot_prepare_identities_or_publish_a_bundle() -> TestResult {
    let directory = TempDir::new()?;
    let root = directory.path().join("store");
    let out = directory.path().join("publication");
    {
        let store = Store::open(&root)?;
        build_corpus(1)?.append(&store)?;
        check!(store
            .scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?
            .is_empty());
    }
    let before = retained_state(&root)?;
    let output = Command::new(env!("CARGO_BIN_EXE_census-service"))
        .current_dir(directory.path())
        .args([
            "run",
            "--school-year",
            "2025",
            "--input",
            "missing-registry.csv",
        ])
        .arg("--store")
        .arg(&root)
        .arg("--out")
        .arg(&out)
        .env("RUST_LOG", "off")
        .env("RUST_BACKTRACE", "0")
        .env("RUST_LIB_BACKTRACE", "0")
        .output()?;
    check!(eq; output.status.code(), Some(1));
    check!(String::from_utf8_lossy(&output.stderr).contains("gathering athletic.net rows"));
    check!(!out.try_exists()?);
    check!(eq; retained_state(&root)?, before);
    check_managed_only(&root, &output)?;
    Ok(())
}

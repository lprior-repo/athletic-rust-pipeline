use super::material::{ks_pass, store_seeded_with_ks};
use super::process::{counters_of, report_of, run_census};
use super::{
    census, digest_of, journal_keys, note, open_store, report, BTreeMap, Derivation, Digest,
    ExportDataset, Sha256, Store, Table, KS_PHASE,
};

#[test]
fn exporter_restart_republishes_identical_snapshots_and_totals() -> super::TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            const SCENARIO: &str = "exporter-restart";
            let dir = tempfile::tempdir()?;
            let root = dir.path().join("store");

            let first = {
                let store = store_seeded_with_ks(&root)?;
                let report = ks_pass(&store, None).await?;
                check!(eq; report.requests, 0);
                export(&store)?
            };
            note(
                SCENARIO,
                format!(
                    "first export: consolidate={:?} snapshots={:?} census_digest={} totals={}",
                    first.counts, first.snapshots, first.census_digest, first.totals
                ),
            );

            let second = {
                let store = open_store(&root)?;
                export(&store)?
            };
            note(
                SCENARIO,
                format!(
            "second export (after reopen): consolidate={:?} snapshots={:?} census_digest={} \
             totals={}",
            second.counts, second.snapshots, second.census_digest, second.totals
        ),
            );

            check!(eq; second.counts, first.counts,
    "a restarted exporter consolidates the same rows");
            check!(eq; second.snapshots, first.snapshots,
    "the published snapshot bytes are identical across the restart");
            check!(eq; second.census_digest, first.census_digest,
    "the census totals are identical across the restart");
            check!(
                !first.snapshots.is_empty(),
                "the corpus produced at least one snapshot to compare"
            );
            Ok(())
        })
}

struct Export {
    counts: Vec<(String, usize)>,
    snapshots: BTreeMap<String, String>,
    census_digest: String,
    totals: String,
}

fn export(store: &Store) -> super::TestResult<Export> {
    let counts: Vec<(String, usize)> = census::consolidate(store)?
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .collect();
    let snapshots = Table::ALL
        .into_iter()
        .filter_map(|table| {
            let bytes =
                std::fs::read(store.out_dir().join(format!("{}.jsonl", table.file()))).ok()?;
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let digest: String = hasher
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            Some((table.file().to_string(), digest))
        })
        .collect();
    let dataset = ExportDataset::load(store)?;
    let derivation = Derivation::of(&dataset, report::Scope::AllSources, None);
    let census = report::build_census(&derivation, &store.out_dir());
    let mut projected = serde_json::to_value(&census)?;
    if let Some(object) = projected.as_object_mut() {
        object.remove("generated_on");
        object.remove("store_dir");
    }
    let census_digest = digest_of(&projected)?;
    let totals = format!(
        "athletes={} schools={} meets={:?}",
        projected
            .pointer("/totals/athletes")
            .cloned()
            .map_or(serde_json::Value::Null, |value| value),
        projected
            .pointer("/totals/schools")
            .cloned()
            .map_or(serde_json::Value::Null, |value| value),
        projected
            .pointer("/meets/total")
            .cloned()
            .map_or(serde_json::Value::Null, core::convert::identity),
    );
    Ok(Export {
        counts,
        snapshots,
        census_digest,
        totals,
    })
}

#[test]
fn cli_worker_restart_across_processes_resumes_and_keeps_counters() -> super::TestResult {
    const SCENARIO: &str = "cli-restart";
    let dir = tempfile::tempdir()?;

    let control_root = dir.path().join("control");
    {
        let store = store_seeded_with_ks(&control_root)?;
        drop(store);
    }
    let control_report = report_of(&run_census(&control_root, &["provider", "ks"])?)?;
    let control_stats = counters_of(&run_census(&control_root, &["fjall-stats"])?);
    let control_journal = {
        let store = open_store(&control_root)?;
        journal_keys(&store, KS_PHASE)?
    };
    let total_units = control_journal.len();
    note(
        SCENARIO,
        format!(
            "control process: rows={} units={} stats={control_stats:?}",
            control_report["rows"], total_units
        ),
    );
    check!(eq; control_report["rows"].as_u64(),
    Some(total_units as u64),
    "the clean process covered every unit");

    let root = dir.path().join("restart");
    {
        let store = store_seeded_with_ks(&root)?;
        drop(store);
    }
    let first = report_of(&run_census(&root, &["provider", "ks", "--limit", "2"])?)?;
    let journal_after_first = {
        let store = open_store(&root)?;
        journal_keys(&store, KS_PHASE)?
    };
    note(
        SCENARIO,
        format!(
            "first process: rows={} requests={} from_cache={} journal={:?}",
            first["rows"], first["requests"], first["from_cache"], journal_after_first
        ),
    );
    check!(eq; journal_after_first.len(),
    2,
    "the first process did two units");
    check!(eq; first["requests"].as_u64(),
    Some(0),
    "the directory request must come from the seeded cache");

    let resumed = report_of(&run_census(&root, &["provider", "ks"])?)?;
    let resumed_stats = counters_of(&run_census(&root, &["fjall-stats"])?);
    let consolidated = counters_of(&run_census(&root, &["consolidate"])?);
    let after_export_stats = counters_of(&run_census(&root, &["fjall-stats"])?);
    let final_journal = {
        let store = open_store(&root)?;
        journal_keys(&store, KS_PHASE)?
    };
    note(
        SCENARIO,
        format!(
            "restart process: rows={} journal={} notes={:?} stats={resumed_stats:?}",
            resumed["rows"],
            final_journal.len(),
            resumed["notes"]
        ),
    );
    note(
        SCENARIO,
        format!(
            "exporter process: consolidate={consolidated:?} stats_after={after_export_stats:?}"
        ),
    );

    check!(eq; resumed["rows"].as_u64(),
    Some(total_units as u64 - 2),
    "the restarted process processed only the units the first one did not journal");
    check!(eq; final_journal, control_journal,
    "the restart completes exactly the control's unit set");
    check!(eq; resumed_stats, control_stats,
    "the store's counters after the restart equal a single-process control");
    check!(eq; after_export_stats, control_stats,
    "consolidating does not change the observation counters");
    check!(eq; consolidated.get("schools"),
    control_stats.get("schools"),
    "the exporter publishes one row per stored school observation");
    Ok(())
}

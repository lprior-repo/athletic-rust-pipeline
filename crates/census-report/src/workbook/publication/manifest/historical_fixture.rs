use super::{hash_artifact, Manifest, Selection};
use crate::export::ExportDataset;
use crate::workbook::publication::{binding, inventory::RowCounts};
use crate::workbook::Options;
use census_domain::model::serialized_digest;
use census_store::{publish::InputFence, Store};
use std::collections::BTreeMap;
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const NORMALIZED: [&str; 8] = [
    "athletes.csv",
    "best_marks.csv",
    "schools.csv",
    "coach_contacts.csv",
    "comparison_classes.csv",
    "provenance.csv",
    "coverage.csv",
    "identity_aliases.csv",
];
const BEST_HEADERS: [&str; 36] = [
    "athlete_id",
    "comparison_class_id",
    "performance_id",
    "performance_version",
    "performance_version_status",
    "event_label",
    "raw_mark",
    "raw_mark_status",
    "display_mark",
    "normalized_mark",
    "unit",
    "precision",
    "timing_class",
    "wind_class",
    "technical_class",
    "meet_id",
    "meet_date",
    "meet_name",
    "course",
    "school_at_performance",
    "school_at_performance_status",
    "source_url",
    "capture_reference",
    "capture_reference_status",
    "evidence_refs",
    "tied_performance_count",
    "eligible_candidate_count",
    "tied_performance_ids",
    "selection_policy_version",
    "surface",
    "input_generation",
    "precision_basis",
    "source_decimal_precision",
    "result_evidence_status",
    "source_url_status",
    "supporting_tied_candidate_ids",
];

pub(in crate::workbook::publication) fn rebind(
    dataset: &mut ExportDataset,
    policy: u32,
) -> TestResult {
    dataset.lineage.policy_revision = policy;
    dataset.lineage.input_generation = serialized_digest(&(
        &dataset.lineage.store_identity,
        &dataset.lineage.source_digest,
        &dataset.lineage.input_digest,
        &dataset.lineage.generated_on,
        &dataset.lineage.export_job,
        dataset.lineage.schema_revision,
        policy,
    ))?;
    Ok(())
}

pub(in crate::workbook::publication) fn input(
    store: &Store,
    dataset: &ExportDataset,
    options: &Options,
    revision: u32,
) -> TestResult<InputFence> {
    let current = ExportDataset::load(store)?;
    let mut input = binding::input(&current, store, options)?;
    check!(eq; current.lineage.source_digest, dataset.lineage.source_digest);
    check!(eq; current.lineage.store_identity, dataset.lineage.store_identity);
    input.token = binding::input_token(&dataset.lineage)?;
    input.recipe = binding::recipe_for_artifact(&dataset.lineage, options, revision)?;
    Ok(input)
}

pub(in crate::workbook::publication) fn render_policy20(
    store: &Store,
    directory: &Path,
    options: &Options,
    revision: u32,
) -> TestResult<(ExportDataset, Manifest)> {
    if !matches!(revision, 3..=5) {
        return Err("historical semantic fixture supports artifact revisions 3 through 5".into());
    }
    let mut dataset = ExportDataset::load(store)?;
    rebind(&mut dataset, 20)?;
    std::fs::create_dir_all(directory)?;
    let censuses =
        crate::workbook::Censuses::of(&dataset, &store.out_dir(), &options.jurisdictions)?;
    let counts = crate::workbook::write_artifacts(directory, &dataset, options, &censuses)?;
    historical_header(directory, revision)?;
    let counts = if revision == 5 {
        Some(counts)
    } else {
        std::fs::remove_file(directory.join("combined_exclusions.csv"))?;
        None
    };
    let manifest = capture(directory, &dataset, options, revision, counts)?;
    crate::workbook::publication::write_json(&directory.join("manifest.json"), &manifest)?;
    Ok((dataset, manifest))
}

fn historical_header(directory: &Path, revision: u32) -> TestResult {
    let path = directory.join("best_marks.csv");
    let mut reader = csv::Reader::from_path(&path)?;
    check!(eq; reader.headers()?.iter().collect::<Vec<_>>(), BEST_HEADERS.to_vec());
    if revision != 3 {
        return Ok(());
    }
    check!(
        reader.records().next().transpose()?.is_none(),
        "v3 reconstruction requires a markless fixture"
    );
    drop(reader);
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record(BEST_HEADERS.iter().take(35))?;
    writer.flush()?;
    Ok(())
}

fn names(options: &Options, revision: u32) -> Vec<String> {
    let cohort = options
        .grad_year
        .map_or_else(|| "all".into(), |year| format!("co{year}"));
    let mut names = vec![
        format!("best-results-{cohort}.csv"),
        format!("best-results-{cohort}.jsonl"),
    ];
    names.extend(
        [
            "audit.json",
            "census-all-sources.json",
            "census-core.json",
            "frozen-input.json",
            "recruiting.csv",
            "school-contacts.csv",
            "contact-research.csv",
            "workbook.xlsx",
            "event-catalog.json",
            "relay-dispositions.json",
            "relays.jsonl",
        ]
        .map(str::to_owned),
    );
    names.extend(NORMALIZED.map(str::to_owned));
    if revision == 5 {
        names.push("combined_exclusions.csv".into());
    }
    names
}

fn capture(
    directory: &Path,
    dataset: &ExportDataset,
    options: &Options,
    revision: u32,
    counts: Option<RowCounts>,
) -> TestResult<Manifest> {
    check!(eq; dataset.lineage.policy_revision, 20);
    let artifacts = names(options, revision)
        .into_iter()
        .map(|name| hash_artifact(&directory.join(&name)).map(|artifact| (name, artifact)))
        .collect::<crate::report::ReportResult<BTreeMap<_, _>>>()?;
    let mut manifest = Manifest {
        schema_revision: revision,
        lineage: dataset.lineage.clone(),
        selection: Selection::of(options),
        artifacts,
        normalized_csv_rows: counts,
        selected_profile: None,
        generation_digest: String::new(),
    };
    manifest.generation_digest = match revision {
        3 | 4 => serialized_digest(&(
            revision,
            &manifest.lineage,
            &manifest.selection,
            &manifest.artifacts,
        ))?,
        5 => serialized_digest(&(
            5_u32,
            &manifest.lineage,
            &manifest.selection,
            &manifest.artifacts,
            &manifest.normalized_csv_rows,
        ))?,
        _ => return Err("unsupported historical fixture digest".into()),
    };
    Ok(manifest)
}

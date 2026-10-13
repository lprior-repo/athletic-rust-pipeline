use super::super::{options, TestResult};
use crate::export::{DatasetLineage, ExportDataset};
use census_domain::model::{serialized_digest, SchoolYear};
use census_store::{Store, Table};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const ORIGINAL_DATA: &str = r#"{"athletes":[],"schools":{},"teams":{},"coaches":[],"coach_observations":[],"events":[],"meets":[],"performances":[],"review_cases":[],"source_access":[],"verdicts":[],"identity_decisions":[],"coverage_evidence":{"rows":[],"coach_lanes":[]}}"#;
const ORIGINAL_TABLES: [Table; 15] = [
    Table::Schools,
    Table::Teams,
    Table::Coaches,
    Table::Athletes,
    Table::Meets,
    Table::Events,
    Table::Performances,
    Table::ReviewCases,
    Table::SourceAccess,
    Table::IdentityVerdicts,
    Table::AthleteIdentityDecisions,
    Table::SourceObservations,
    Table::SourceMeets,
    Table::Coverage,
    Table::CoachLanes,
];

#[derive(Serialize)]
struct Selection {
    scope: &'static str,
    grad_year: Option<i16>,
    limit: Option<usize>,
    school_year: Option<SchoolYear>,
}

#[derive(Serialize)]
struct Artifact {
    bytes: u64,
    sha256: String,
}

#[derive(Serialize)]
struct Manifest<'a> {
    schema_revision: u32,
    lineage: &'a DatasetLineage,
    selection: Selection,
    artifacts: BTreeMap<String, Artifact>,
    generation_digest: String,
}

pub(super) fn install(store: &Store, root: &Path) -> TestResult<PathBuf> {
    let mut lineage = ExportDataset::load(store)?.lineage.clone();
    lineage.schema_revision = 2;
    lineage.policy_revision = 13;
    lineage.source_digest = store.snapshot().tables_digest(&ORIGINAL_TABLES)?;
    lineage.input_digest = format!("{:x}", Sha256::digest(ORIGINAL_DATA.as_bytes()));
    lineage.input_generation = serialized_digest(&(
        &lineage.store_identity,
        &lineage.source_digest,
        &lineage.input_digest,
        &lineage.generated_on,
        &lineage.export_job,
        2_u32,
        13_u32,
    ))?;
    let stage = root.join("reconstructed-policy13-integrity-fixture");
    std::fs::create_dir_all(root.join("generations"))?;
    std::fs::create_dir(&stage)?;
    write_literals(&stage, &lineage)?;
    let artifacts = bytes(&stage)?
        .into_iter()
        .map(|(name, bytes)| {
            Ok((
                name,
                Artifact {
                    bytes: u64::try_from(bytes.len())?,
                    sha256: format!("{:x}", Sha256::digest(&bytes)),
                },
            ))
        })
        .collect::<TestResult<BTreeMap<_, _>>>()?;
    let selection = Selection {
        scope: "all_sources",
        grad_year: Some(2027),
        limit: None,
        school_year: Some(options()?.school_year),
    };
    let digest = serialized_digest(&(1_u32, &lineage, &selection, &artifacts))?;
    let manifest = Manifest {
        schema_revision: 1,
        lineage: &lineage,
        selection,
        artifacts,
        generation_digest: digest.clone(),
    };
    std::fs::write(stage.join("manifest.json"), serde_json::to_vec(&manifest)?)?;
    let retained = root.join("generations").join(&digest);
    std::fs::rename(&stage, &retained)?;
    std::os::unix::fs::symlink(Path::new("generations").join(digest), root.join("current"))?;
    Ok(retained)
}

fn write_literals(directory: &Path, lineage: &DatasetLineage) -> TestResult {
    let frozen = format!(
        "{{\"lineage\":{},\"data\":{ORIGINAL_DATA}}}",
        serde_json::to_string(lineage)?
    );
    std::fs::write(directory.join("frozen-input.json"), frozen)?;
    for (name, content) in [
        ("audit.json", "{}"),
        ("best-results-co2027.csv", "Athlete ID\n"),
        ("best-results-co2027.jsonl", ""),
        ("census-all-sources.json", "{}"),
        ("census-core.json", "{}"),
        ("recruiting.csv", "Athlete ID\n"),
        ("school-contacts.csv", "School ID\n"),
        ("contact-research.csv", "School ID\n"),
    ] {
        std::fs::write(directory.join(name), content)?;
    }
    let mut book = rust_xlsxwriter::Workbook::new();
    for name in [
        "Athletes",
        "PRs",
        "Performances_001",
        "Coaches",
        "Schools",
        "Meets",
        "Sources",
        "Coverage",
        "Conflicts",
        "Review",
        "Run Metrics",
        "School Contacts",
        "Contact Research",
    ] {
        book.add_worksheet().set_name(name)?;
    }
    book.save(directory.join("workbook.xlsx"))?;
    Ok(())
}

pub(super) fn bytes(directory: &Path) -> TestResult<BTreeMap<String, Vec<u8>>> {
    std::fs::read_dir(directory)?
        .map(|entry| {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "non-UTF8 fixture path")?;
            Ok((name, std::fs::read(entry.path())?))
        })
        .collect()
}

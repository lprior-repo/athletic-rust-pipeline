use anyhow::{bail, Context, Result};
use athletic_rust_pipeline::{
    domain::{
        decision::{assess, SearchCompleteness},
        evidence::Sport,
        identity::{AthleteId, EvidenceDigest, SourceRowKey, WorkbookDigest},
    },
    model::{SourceRecord, SOURCE_HEADERS},
    runtime::import::SourceManifest,
    search::SearchQuery,
    store::ArtifactStore,
    workbook_export::{ExportRow, WorkbookExport},
};
use rust_xlsxwriter::Workbook;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use tempfile::{tempdir, TempDir};

pub const ROWS_PER_SHEET: u32 = 48;
pub const RESULT_COUNT: usize = 64;
pub const DIGEST_HEX: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

pub struct Fixture {
    pub _directory: TempDir,
    pub workbook: PathBuf,
    pub output: PathBuf,
    pub digest: WorkbookDigest,
    pub evidence_digest: EvidenceDigest,
    pub records: Vec<SourceRecord>,
    pub export_rows: Vec<ExportRow>,
    pub manifest: SourceManifest,
    pub store: ArtifactStore,
    pub document_digest: EvidenceDigest,
    pub source_key: SourceRowKey,
    pub search_query: SearchQuery,
    pub search_body: Vec<u8>,
    pub profile_html: Vec<u8>,
    pub profile_bio: Vec<u8>,
    pub profile: athletic_rust_pipeline::domain::evidence::ProfileEvidence,
}

impl Fixture {
    pub fn new() -> Result<Self> {
        let directory = tempdir().context("creating benchmark fixture directory")?;
        let workbook = directory.path().join("source.xlsx");
        write_workbook(&workbook)?;
        let digest = workbook_digest(&workbook)?;
        let frozen = directory.path().join("frozen.xlsx");
        fs::copy(&workbook, &frozen).context("copying frozen workbook")?;
        let records = source_records();
        let stats = workbook_stats();
        let manifest = SourceManifest {
            original: workbook.clone(),
            frozen,
            workbook: digest.clone(),
            ingestion_revision: "bench-fixture-v1".to_owned(),
            stats,
        };
        let extra_headers = vec!["Fixture Scenario".to_owned()];
        let export_rows = records
            .iter()
            .cloned()
            .map(|source| ExportRow {
                source,
                extra_fields: BTreeMap::from([(
                    "Fixture Scenario".to_owned(),
                    "synthetic".to_owned(),
                )]),
            })
            .collect::<Vec<_>>();
        let output = directory.path().join("export.xlsx");
        run_export(&manifest, &export_rows, &output, &extra_headers)?;
        let store = ArtifactStore::open(&directory.path().join("artifacts"))?;
        store.put_source_batch(&digest, &records)?;
        let document_digest = store.put_bytes(b"synthetic retained profile document")?;
        let source_key = SourceRowKey::parse("Alpha:2")?;
        let evidence_digest = EvidenceDigest::parse(DIGEST_HEX)?;
        let search_query = SearchQuery::new("Synthetic Runner", Sport::TrackField, 0)?;
        let search_body = search_body()?;
        let profile_html = profile_html();
        let profile_bio = profile_bio()?;
        let athlete = AthleteId::new(123)?;
        let profile = athletic_rust_pipeline::profile::parse_bio(
            athlete,
            Sport::TrackField,
            evidence_digest.clone(),
            &profile_bio,
        )?;
        let source = records
            .first()
            .context("benchmark fixture has no source records")?;
        let assessment = assess(
            source,
            std::slice::from_ref(&profile),
            SearchCompleteness::Complete {
                evidence: evidence_digest.clone(),
            },
        )?;
        if !assessment.is_deterministic_acceptance()
            || assessment.accepted_athlete_id() != Some(athlete)
        {
            bail!("benchmark fixture must deterministically accept athlete 123");
        }
        Ok(Self {
            _directory: directory,
            workbook,
            output,
            digest,
            evidence_digest,
            records,
            export_rows,
            manifest,
            store,
            document_digest,
            source_key,
            search_query,
            search_body,
            profile_html,
            profile_bio,
            profile,
        })
    }
}

pub fn run_export(
    manifest: &SourceManifest,
    rows: &[ExportRow],
    destination: &Path,
    extra_headers: &[String],
) -> Result<()> {
    let mut export = WorkbookExport::new(manifest, extra_headers)?;
    rows.iter().cloned().try_for_each(|row| export.write(row))?;
    export.finish(destination).map(|_| ())
}

fn write_workbook(path: &Path) -> Result<()> {
    let mut workbook = Workbook::new();
    write_sheet(&mut workbook, "Alpha")?;
    write_sheet(&mut workbook, "Beta")?;
    workbook.save(path).context("saving synthetic workbook")?;
    Ok(())
}

fn write_sheet(workbook: &mut Workbook, name: &str) -> Result<()> {
    let worksheet = workbook.add_worksheet_with_constant_memory();
    worksheet.set_name(name).context("naming fixture sheet")?;
    SOURCE_HEADERS
        .iter()
        .enumerate()
        .try_for_each(|(column, header)| {
            let column = u16::try_from(column).context("header column overflow")?;
            worksheet
                .write_string(0, column, *header)
                .map(|_| ())
                .context("writing fixture header")
        })?;
    (2..=ROWS_PER_SHEET + 1).try_for_each(|row| {
        let index = row - 2;
        fixture_values(index)
            .iter()
            .enumerate()
            .try_for_each(|(column, value)| {
                let column = u16::try_from(column).context("data column overflow")?;
                worksheet
                    .write_string(row - 1, column, value)
                    .map(|_| ())
                    .context("writing fixture value")
            })
    })
}

fn fixture_values(index: u32) -> Vec<String> {
    vec![
        "Synthetic".to_owned(),
        "Runner".to_owned(),
        format!("runner{index}@example.test"),
        format!("{index} Fictional Way"),
        "Fictional City".to_owned(),
        "CA".to_owned(),
        format!("900{:02}", index % 100),
        "2026-01-01".to_owned(),
        "Track and Field".to_owned(),
        "100".to_owned(),
        "2026-06-01".to_owned(),
        "synthetic-source".to_owned(),
        "Fictional High".to_owned(),
    ]
}

fn source_records() -> Vec<SourceRecord> {
    ["Alpha", "Beta"]
        .into_iter()
        .flat_map(|sheet| {
            (2..=ROWS_PER_SHEET + 1).map(move |excel_row| {
                let index = excel_row - 2;
                let fields = SOURCE_HEADERS
                    .iter()
                    .zip(fixture_values(index))
                    .map(|(header, value)| ((*header).to_owned(), value))
                    .collect();
                SourceRecord {
                    source_key: format!("{sheet}:{excel_row}"),
                    sheet: sheet.to_owned(),
                    excel_row,
                    fields,
                }
            })
        })
        .collect()
}

fn workbook_stats() -> athletic_rust_pipeline::model::WorkbookStats {
    let rows = u64::from(ROWS_PER_SHEET);
    athletic_rust_pipeline::model::WorkbookStats {
        sheets: ["Alpha", "Beta"]
            .into_iter()
            .map(|name| athletic_rust_pipeline::model::SheetStats {
                name: name.to_owned(),
                declared_dimension: Some(format!("A1:M{}", ROWS_PER_SHEET + 1)),
                xml_rows: rows + 1,
                actual_data_rows: rows,
                last_actual_row: ROWS_PER_SHEET + 1,
                headers: SOURCE_HEADERS
                    .iter()
                    .map(|header| (*header).to_owned())
                    .collect(),
            })
            .collect(),
        actual_data_rows: rows * 2,
        selected_prospects: 0,
    }
}

fn workbook_digest(path: &Path) -> Result<WorkbookDigest> {
    let bytes = fs::read(path).context("reading fixture workbook")?;
    let hex = format!("{:x}", Sha256::digest(bytes));
    WorkbookDigest::parse(&hex).map_err(Into::into)
}

fn search_body() -> Result<Vec<u8>> {
    let rows = (0..RESULT_COUNT)
        .map(|index| {
            format!(
                "<tr><td><a href='/athlete/{}/track-and-field'>Synthetic Runner {index}</a> Fictional High</td></tr>",
                index + 1
            )
        })
        .collect::<String>();
    let body = serde_json::json!({
        "d": {
            "count": RESULT_COUNT,
            "pager": "",
            "runTime": "0.42",
            "results": rows,
        }
    });
    serde_json::to_vec(&body).context("encoding search fixture")
}

fn profile_html() -> Vec<u8> {
    br#"<link rel="canonical" href="https://www.athletic.net/athlete/123/track-and-field/all"><div data-athlete-id="123"><span>Class of </span><strong>2027</strong></div><script>window.anetSiteAppParams={"tree":[{"type":"athlete","id":123,"title":"Synthetic Runner"}]};</script>"#.to_vec()
}

fn profile_bio() -> Result<Vec<u8>> {
    let results = (0..RESULT_COUNT)
        .map(|index| {
            serde_json::json!({
                "IDResult": index + 1,
                "AthleteID": 123,
                "Result": format!("10.{:02}", 50 - (index % 40)),
                "SchoolID": 7,
                "MeetID": index + 10,
                "SeasonID": 12026,
                "EventID": 1,
                "PersonalBest": 1,
                "SeasonBest": 1,
                "FAT": 1,
                "shortCode": format!("synthetic-{index}"),
            })
        })
        .collect::<Vec<_>>();
    let meets = (0..RESULT_COUNT)
        .map(|index| {
            (
                (index + 10).to_string(),
                serde_json::json!({"MeetName": "Synthetic Meet"}),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let body = serde_json::json!({
        "athlete": {"IDAthlete": 123, "FirstName": "Synthetic", "LastName": "Runner"},
        "allSeasons": [{"SchoolID": 7, "IDSeason": 12026}],
        "allTeams": {"7": {"SchoolName": "Fictional High", "City": "Fictional City", "State": "CA", "Level": 4}},
        "grades": {"7_12026": 12},
        "meets": meets,
        "resultsTF": results,
        "eventsTF": [{"IDEvent": 1, "Event": "100 Meters", "Type": "T", "PersonalEvent": true}],
        "resultsXC": null,
    });
    serde_json::to_vec(&body).context("encoding profile fixture")
}

pub fn ready<T, E>(result: std::result::Result<T, E>) -> T
where
    E: std::fmt::Display,
{
    match result {
        Ok(value) => value,
        Err(error) => {
            eprintln!("benchmark fixture setup failed: {error}");
            std::process::exit(1);
        }
    }
}

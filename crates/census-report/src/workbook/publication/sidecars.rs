use crate::export::ExportDataset;
use crate::report::{io_error, Derivation, ReportError, ReportResult};
use crate::workbook::{Censuses, Options};
use census_domain::model::{CanonicalPerformance, SchoolYear};
use census_store::Entity;
use serde::{Serialize, Serializer};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

#[derive(Serialize)]
struct Audit<'a> {
    input_generation: &'a str,
    scope: &'static str,
    grad_year: Option<i16>,
    school_year: SchoolYear,
    athletes: RecordIds<'a, census_domain::model::CanonicalAthlete>,
    performances: PerformanceIds<'a>,
    coaches: RecordIds<'a, census_domain::model::CanonicalCoach>,
    identity_decisions: RecordIds<'a, census_domain::model::AppliedAthleteIdentity>,
}

struct RecordIds<'a, T>(&'a [T]);
impl<T: Entity> Serialize for RecordIds<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(Entity::entity_id))
    }
}

struct PerformanceIds<'a>(&'a [&'a CanonicalPerformance]);
impl Serialize for PerformanceIds<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(|row| row.id.as_str()))
    }
}

pub(in crate::workbook) fn write_sidecars(
    directory: &Path,
    dataset: &ExportDataset,
    _options: &Options,
    censuses: &Censuses,
    derivation: &Derivation<'_>,
    school_year: SchoolYear,
) -> ReportResult<()> {
    write_json(&directory.join("census-core.json"), &censuses.core)?;
    write_json(
        &directory.join("census-all-sources.json"),
        &censuses.all_sources,
    )?;
    crate::workbook::write_recruiting_csv(
        derivation,
        school_year,
        &directory.join("recruiting.csv"),
    )?;
    write_json(
        &directory.join("audit.json"),
        &Audit {
            input_generation: &dataset.lineage.input_generation,
            scope: derivation.scope().as_str(),
            grad_year: derivation.grad_year(),
            school_year,
            athletes: RecordIds(derivation.athletes()),
            performances: PerformanceIds(derivation.performances()),
            coaches: RecordIds(derivation.coach_observations()),
            identity_decisions: RecordIds(&dataset.identity_decisions),
        },
    )
}

pub(super) fn write_json(path: &Path, value: &impl Serialize) -> ReportResult<()> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| io_error(path, source))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, value).map_err(|source| ReportError::Decode {
        path: path.to_path_buf(),
        line: 0,
        source,
    })?;
    writer.flush().map_err(|source| io_error(path, source))?;
    writer
        .get_ref()
        .sync_all()
        .map_err(|source| io_error(path, source))
}

pub(super) fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> ReportResult<T> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
    if !metadata.file_type().is_file() || metadata.len() > 1024 * 1024 {
        return Err(ReportError::Invariant {
            detail: "invalid or oversized generation manifest".to_string(),
        });
    }
    let file = std::fs::File::open(path).map_err(|source| io_error(path, source))?;
    serde_json::from_reader(BufReader::new(file)).map_err(|source| ReportError::Decode {
        path: path.to_path_buf(),
        line: 0,
        source,
    })
}

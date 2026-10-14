use crate::bests::SharedSelection;
use crate::export::ExportDataset;
use crate::report::{Derivation, EventAdmission, ReportError, ReportResult};
use crate::workbook::Options;
use census_store::read::csv_failure;
use std::path::Path;

mod athletes;
mod best_marks;
mod coach_contacts;
mod comparison_classes;
mod coverage;
mod identity_aliases;
mod integrity;
mod provenance;
mod schools;
#[cfg(test)]
mod tests;

type PerformanceIndex<'a> = std::collections::BTreeMap<
    &'a census_domain::model::PerformanceId,
    &'a census_domain::model::CanonicalPerformance,
>;

pub(in crate::workbook) fn write(
    directory: &Path,
    dataset: &ExportDataset,
    options: &Options,
    bests: &[SharedSelection],
) -> ReportResult<()> {
    let admission = EventAdmission::of(dataset)?;
    let derivation = Derivation::of(
        dataset,
        options.scope,
        options.grad_year,
        &options.jurisdictions,
    );
    let performances = integrity::validate(&derivation, &admission, bests)?;
    let schools = schools::included(&derivation, bests, &performances)?;
    census_store::fs::create_dir_all_synced(directory)
        .map_err(|source| crate::report::io_error(directory, source))?;
    athletes::write(directory, &derivation, bests)?;
    best_marks::write(directory, &derivation, bests, &performances)?;
    schools::write(directory, &schools, &dataset.lineage.input_generation)?;
    coach_contacts::write(directory, &derivation, options.school_year)?;
    comparison_classes::write(directory, dataset, bests)?;
    provenance::write(directory, &derivation, &schools, bests, &performances)?;
    coverage::write(directory, dataset)?;
    identity_aliases::write(directory, &derivation)
}

fn write_csv(
    path: &Path,
    header: &[&str],
    body: impl FnOnce(&mut Rows<'_>) -> ReportResult<()>,
) -> ReportResult<()> {
    let mut failure = None;
    let published = census_store::read::publish_atomically(path, |temporary| {
        let result = write_body(temporary, header, body);
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                let detail = error.to_string();
                failure = Some(error);
                Err(census_store::StoreError::Invariant { detail })
            }
        }
    });
    match failure {
        Some(error) => Err(error),
        None => published.map_err(ReportError::from),
    }
}

fn write_body(
    path: &Path,
    header: &[&str],
    body: impl FnOnce(&mut Rows<'_>) -> ReportResult<()>,
) -> ReportResult<()> {
    let writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(path)
        .map_err(|error| csv_failure(path, error))?;
    let mut rows = Rows {
        writer,
        path,
        admission: crate::grouping::Admission::new("normalized CSV"),
    };
    rows.writer
        .write_record(header)
        .map_err(|error| csv_failure(path, error))?;
    body(&mut rows)?;
    rows.writer
        .flush()
        .map_err(|source| crate::report::io_error(path, source))
}

struct Rows<'a> {
    writer: csv::Writer<std::fs::File>,
    path: &'a Path,
    admission: crate::grouping::Admission,
}

impl Rows<'_> {
    fn push<const N: usize>(&mut self, fields: [String; N]) -> ReportResult<()> {
        self.admission.record(&fields.as_slice())?;
        for field in fields {
            let field = crate::csv_safety::protect_owned(field)
                .map_err(|error| defect(format!("protecting CSV field: {error}")))?;
            self.writer
                .write_field(field.as_str())
                .map_err(|error| csv_failure(self.path, error))?;
        }
        self.writer
            .write_record(std::iter::empty::<&str>())
            .map_err(|error| csv_failure(self.path, error))?;
        Ok(())
    }
}

fn encode(value: &impl serde::Serialize) -> ReportResult<String> {
    serde_json::to_string(value)
        .map_err(|error| defect(format!("encoding normalized fact: {error}")))
}

fn digest(value: &impl serde::Serialize) -> ReportResult<String> {
    census_domain::model::serialized_digest(value)
        .map_err(|error| defect(format!("identifying normalized fact: {error}")))
}

fn text(value: Option<&str>) -> String {
    match value.filter(|value| !value.is_empty()) {
        Some(value) => value.to_owned(),
        None => "unavailable".to_owned(),
    }
}

fn defect(detail: impl Into<String>) -> ReportError {
    ReportError::Invariant {
        detail: detail.into(),
    }
}

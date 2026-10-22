use crate::bests::SharedSelection;
#[cfg(test)]
use crate::export::ExportDataset;
use crate::report::{Derivation, EventAdmission, ReportError, ReportResult};
use crate::workbook::Options;
use census_store::read::csv_failure;
use std::path::Path;

mod athletes;
mod best_marks;
mod coach_contacts;
mod combined_exclusions;
mod comparison_classes;
mod coverage;
mod identity_aliases;
mod integrity;
mod provenance;
pub(in crate::workbook::publication) mod reported_performances;
mod schools;
#[cfg(test)]
mod tests;

type PerformanceIndex<'a> = std::collections::BTreeMap<
    &'a census_domain::model::PerformanceId,
    &'a census_domain::model::CanonicalPerformance,
>;

#[cfg(test)]
pub(in crate::workbook) fn write(
    directory: &Path,
    dataset: &ExportDataset,
    options: &Options,
    bests: &[SharedSelection],
    exclusions: &[crate::bests::CombinedExclusion],
) -> ReportResult<super::inventory::RowCounts> {
    if dataset.selected_profile_id().is_some() {
        return Err(defect(
            "selected participation policy requires verified frozen evidence",
        ));
    }
    crate::bests::participation::ParticipationSelection::of(dataset)?;
    let derivation = Derivation::of(
        dataset,
        options.scope,
        options.grad_year,
        &options.jurisdictions,
    )?;
    write_rows(directory, &derivation, options, bests, exclusions)
}

pub(in crate::workbook) fn write_outcome(
    directory: &Path,
    derivation: &Derivation<'_>,
    options: &Options,
    outcome: &crate::bests::SelectionOutcome,
) -> ReportResult<super::inventory::RowCounts> {
    let dataset = derivation.dataset();
    let revision = super::selected::revision(dataset)?;
    derivation.ensure_selected_affiliation()?;
    if derivation.scope() != options.scope
        || derivation.grad_year() != options.grad_year
        || derivation.jurisdictions() != options.jurisdictions
    {
        return Err(defect(
            "normalized selection differs from its bound derivation",
        ));
    }
    if !matches!(
        revision,
        super::selected::REVISION | super::selected::REPORTED_REVISION
    ) && !outcome.participation_exclusions.is_empty()
    {
        return Err(defect(
            "historical output cannot discard participation exclusions",
        ));
    }
    let mut counts = write_rows(
        directory,
        derivation,
        options,
        &outcome.selections,
        &outcome.combined_exclusions,
    )?;
    if matches!(
        revision,
        super::selected::REVISION | super::selected::REPORTED_REVISION
    ) {
        counts.insert(
            "current_affiliations.csv".into(),
            super::current_affiliations::write(directory, dataset)?,
        );
        counts.insert(
            "participation_exclusions.csv".into(),
            super::participation_exclusions::write(
                directory,
                dataset,
                dataset.verified_participation()?,
                outcome,
            )?,
        );
        if revision == super::selected::REPORTED_REVISION {
            let admission = EventAdmission::of(dataset)?;
            let projection = crate::workbook::reported::Projection::of(
                dataset,
                derivation,
                &admission,
                &outcome.selections,
            )?
            .ok_or_else(|| defect("revision 9 lacks Include reported projection"))?;
            counts.insert(
                "reported_performances.csv".into(),
                reported_performances::write(directory, &projection)?,
            );
        }
    }
    Ok(counts)
}

fn write_rows(
    directory: &Path,
    derivation: &Derivation<'_>,
    options: &Options,
    bests: &[SharedSelection],
    exclusions: &[crate::bests::CombinedExclusion],
) -> ReportResult<super::inventory::RowCounts> {
    let dataset = derivation.dataset();
    crate::workbook::public_identity::preflight(dataset, derivation.athletes())?;
    let admission = EventAdmission::of(dataset)?;
    let performances = integrity::validate(derivation, &admission, bests)?;
    let schools = schools::included(derivation, bests, &performances)?;
    census_store::fs::create_dir_all_synced(directory)
        .map_err(|source| crate::report::io_error(directory, source))?;
    Ok(std::collections::BTreeMap::from([
        (
            "athletes.csv".into(),
            athletes::write(directory, derivation, bests)?,
        ),
        (
            "best_marks.csv".into(),
            best_marks::write(directory, derivation, bests, &performances)?,
        ),
        (
            "schools.csv".into(),
            schools::write(directory, &schools, &dataset.lineage.input_generation)?,
        ),
        (
            "coach_contacts.csv".into(),
            coach_contacts::write(directory, derivation, options.school_year)?,
        ),
        (
            "comparison_classes.csv".into(),
            comparison_classes::write(directory, dataset, bests)?,
        ),
        (
            "provenance.csv".into(),
            provenance::write(directory, derivation, &schools, bests, &performances)?,
        ),
        ("coverage.csv".into(), coverage::write(directory, dataset)?),
        (
            "identity_aliases.csv".into(),
            identity_aliases::write(directory, derivation)?,
        ),
        (
            "combined_exclusions.csv".into(),
            combined_exclusions::write(directory, dataset, exclusions, &performances)?,
        ),
    ]))
}

pub(in crate::workbook::publication) fn write_csv(
    path: &Path,
    header: &[&str],
    body: impl FnOnce(&mut Rows<'_>) -> ReportResult<()>,
) -> ReportResult<u64> {
    let mut count = 0;
    let mut failure = None;
    let published = census_store::read::publish_atomically(path, |temporary| {
        let result = write_body(temporary, header, body);
        match result {
            Ok(rows) => {
                count = rows;
                Ok(())
            }
            Err(error) => {
                let detail = error.to_string();
                failure = Some(error);
                Err(census_store::StoreError::Invariant { detail })
            }
        }
    });
    match failure {
        Some(error) => Err(error),
        None => published.map(|()| count).map_err(ReportError::from),
    }
}

fn write_body(
    path: &Path,
    header: &[&str],
    body: impl FnOnce(&mut Rows<'_>) -> ReportResult<()>,
) -> ReportResult<u64> {
    let writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(path)
        .map_err(|error| csv_failure(path, error))?;
    let mut rows = Rows {
        writer,
        path,
        admission: crate::grouping::Admission::new("normalized CSV"),
        count: 0,
    };
    rows.writer
        .write_record(header)
        .map_err(|error| csv_failure(path, error))?;
    body(&mut rows)?;
    rows.writer
        .flush()
        .map_err(|source| crate::report::io_error(path, source))?;
    Ok(rows.count)
}

pub(in crate::workbook::publication) struct Rows<'a> {
    count: u64,
    writer: csv::Writer<std::fs::File>,
    path: &'a Path,
    admission: crate::grouping::Admission,
}

impl Rows<'_> {
    pub(in crate::workbook::publication) fn push<const N: usize>(
        &mut self,
        fields: [String; N],
    ) -> ReportResult<()> {
        self.admission.record(&fields.as_slice())?;
        self.write_fields(fields)
    }

    fn push_vec(&mut self, fields: Vec<String>) -> ReportResult<()> {
        self.admission.record(&fields.as_slice())?;
        self.write_fields(fields)
    }

    fn write_fields(&mut self, fields: impl IntoIterator<Item = String>) -> ReportResult<()> {
        let count = self
            .count
            .checked_add(1)
            .ok_or_else(|| defect("normalized CSV row counter overflow"))?;
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
        self.count = count;
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

use crate::export::{ExportDataset, VerifiedCurrentAffiliations};
use crate::grouping::Admission;
use crate::report::{ReportError, ReportResult};
use census_domain::model::AthleteId;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

use super::verify_sidecars::normalized::rows;

const FILE: &str = "current_affiliations.csv";
const HEADER: [&str; 8] = [
    "retained_candidate_id",
    "current_school_id",
    "resolution_status",
    "evidence_refs",
    "unavailable_reasons",
    "profile_id",
    "as_of",
    "input_generation",
];

pub(super) fn write(directory: &Path, dataset: &ExportDataset) -> ReportResult<u64> {
    let affiliations = dataset.verified_current_affiliations()?;
    let athletes = subjects(dataset)?;
    let mut budget = Admission::new("current affiliation publication");
    super::normalized::write_csv(&directory.join(FILE), &HEADER, |writer| {
        athletes.values().try_for_each(|athlete| {
            writer.push(record(athlete, affiliations, &mut budget)?.fields()?)
        })
    })
}

pub(super) fn verify(directory: &Path, dataset: &ExportDataset) -> ReportResult<u64> {
    let affiliations = dataset.verified_current_affiliations()?;
    let athletes = subjects(dataset)?;
    let mut budget = Admission::new("current affiliation readback");
    rows::table(
        directory,
        (FILE, &HEADER),
        athletes.len(),
        &["retained_candidate_id"],
        |found| {
            let id = found.get("retained_candidate_id")?;
            let athlete = athletes
                .get(id)
                .ok_or_else(|| defect("current affiliation references a foreign athlete"))?;
            let fields = record(athlete, affiliations, &mut budget)?.fields()?;
            HEADER
                .iter()
                .zip(fields)
                .try_for_each(|(column, expected)| found.text(column, expected))
        },
    )
}

const SUBJECT_ENTRY_BYTES: usize = std::mem::size_of::<(&str, &AthleteId)>() * 4;

fn subjects(dataset: &ExportDataset) -> ReportResult<BTreeMap<&str, &AthleteId>> {
    let mut athletes = BTreeMap::new();
    let mut budget = Admission::new("current affiliation subject inventory");
    for athlete in &dataset.athletes {
        budget.record(&athlete.id)?;
        budget.charge(SUBJECT_ENTRY_BYTES, 0)?;
        if athletes.insert(athlete.id.as_str(), &athlete.id).is_some() {
            return Err(defect(
                "current affiliation inventory contains a duplicate athlete",
            ));
        }
    }
    Ok(athletes)
}

#[derive(Serialize)]
struct Record<'a> {
    athlete_id: &'a str,
    current_school_id: &'a str,
    resolution_status: &'static str,
    evidence_refs: &'a [String],
    unavailable_reasons: Vec<&'a str>,
    profile_id: &'a str,
    as_of: &'a str,
    input_generation: &'a str,
}

impl Record<'_> {
    fn fields(self) -> ReportResult<[String; 8]> {
        Ok([
            self.athlete_id.into(),
            self.current_school_id.into(),
            self.resolution_status.into(),
            encode(&self.evidence_refs)?,
            encode(&self.unavailable_reasons)?,
            self.profile_id.into(),
            self.as_of.into(),
            self.input_generation.into(),
        ])
    }
}

fn record<'a>(
    athlete: &'a AthleteId,
    affiliations: &'a VerifiedCurrentAffiliations,
    budget: &mut Admission,
) -> ReportResult<Record<'a>> {
    let resolution = affiliations.resolution(athlete);
    let audit = affiliations.audit(athlete);
    let row = Record {
        athlete_id: athlete.as_str(),
        current_school_id: resolution.school().map_or("", |school| school.as_str()),
        resolution_status: resolution.status(),
        evidence_refs: audit.map_or(&[], |audit| audit.references()),
        unavailable_reasons: audit.map_or_else(
            || vec!["missing_current_affiliation_evidence"],
            |audit| audit.unavailable().iter().map(String::as_str).collect(),
        ),
        profile_id: affiliations.profile_id().as_str(),
        as_of: affiliations.as_of().as_str(),
        input_generation: affiliations.input_generation(),
    };
    budget.record(&row)?;
    Ok(row)
}

fn encode(value: &impl Serialize) -> ReportResult<String> {
    serde_json::to_string(value)
        .map_err(|error| defect(format!("encoding current affiliation: {error}")))
}

fn defect(detail: impl Into<String>) -> ReportError {
    ReportError::Invariant {
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(in crate::workbook::publication) mod fixture;

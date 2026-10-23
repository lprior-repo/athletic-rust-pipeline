use super::queues::{conflict_families, review_families};
use super::{school_name_index, Family, QueueRow, StoreRows};
use census_domain::model::{GradYear, SchoolYear};
use census_domain::UsJurisdiction;

use crate::report::{ReportResult, Scope};

pub fn retained_records(
    dataset: &crate::export::ExportDataset,
    school_year: SchoolYear,
    jurisdictions: &[UsJurisdiction],
) -> ReportResult<RetainedRecords> {
    let admission = crate::report::EventAdmission::of(dataset)?;
    let derivation = crate::report::Derivation::for_publication(
        dataset,
        Scope::AllSources,
        None,
        jurisdictions,
    )?;
    let rows = StoreRows::of(&derivation, school_year)?;
    let cohort = crate::report::cohort_candidates(
        dataset,
        Scope::AllSources,
        Some(GradYear::CO2027.get()),
        jurisdictions,
    )?;
    let names = school_name_index(rows.schools);
    Ok(RetainedRecords {
        conflicts: labelled(conflict_families(&rows, &cohort, &names)?),
        reviews: labelled(review_families(&rows, &cohort, &names, &admission)?),
    })
}

#[derive(Debug, Default)]
pub struct RetainedRecords {
    pub conflicts: Vec<(&'static str, QueueRow)>,
    pub reviews: Vec<(&'static str, QueueRow)>,
}

fn labelled(families: Vec<Family>) -> Vec<(&'static str, QueueRow)> {
    let mut out = Vec::new();
    for family in families {
        for row in family.rows {
            out.push((family.label, row));
        }
    }
    out
}

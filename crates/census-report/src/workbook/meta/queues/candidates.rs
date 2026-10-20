use crate::export::public_identity::population::{IdentityAdmission, IdentityPopulation};
use crate::export::ExportDataset;
use crate::report::ReportResult;

use super::super::Family;
use super::{queue_row, resolution_state};

pub(super) fn withheld(dataset: &ExportDataset) -> ReportResult<Family> {
    let population = IdentityPopulation::of(dataset)?;
    let view = dataset.verified_public_identities()?;
    let mut family = Family::new("Public identity candidate withheld");
    for athlete in &dataset.athletes {
        if population.admission(athlete.id.as_str()) == IdentityAdmission::Accepted {
            continue;
        }
        let state = resolution_state(view.candidate_resolution(athlete.id.as_str()));
        family.push(queue_row(
            athlete.id.as_str(),
            athlete.canonical_name.clone(),
            format!(
                "retained candidate record; public identity resolution {state}; excluded from accepted people counts; this row does not establish a distinct person"
            ),
        ));
    }
    Ok(family)
}

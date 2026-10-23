use super::super::super::{school_of, subject_of, Family};
use super::super::{queue_row, ATHLETE_GENDER, SCHOOL_ADDRESS, TRANSFER};
use census_domain::model::{CanonicalAthlete, CanonicalSchool, Gender};
use std::collections::HashMap;

pub(in crate::workbook::meta::queues) fn gender_unverified(
    cohort: &[CanonicalAthlete],
    names: &HashMap<&str, &str>,
) -> Family {
    let mut family = Family::new(ATHLETE_GENDER);
    for athlete in cohort {
        if athlete.gender != Gender::Unknown {
            continue;
        }
        family.push(queue_row(
            athlete.id.as_str(),
            subject_of(
                &athlete.canonical_name,
                school_of(names, athlete.school.as_str()),
            ),
            "no source supplied a boys or girls competition category; retained gender is Unknown"
                .to_string(),
        ));
    }
    family
}

pub(in crate::workbook::meta::queues) fn address_unverified(
    cohort: &[CanonicalAthlete],
    schools: &[CanonicalSchool],
    names: &HashMap<&str, &str>,
) -> Family {
    let mut addressed = HashMap::new();
    for school in schools {
        addressed.insert(school.id.as_str(), !school.postal_addresses.is_empty());
    }
    let mut family = Family::new(SCHOOL_ADDRESS);
    for athlete in cohort {
        if addressed
            .get(athlete.school.as_str())
            .is_some_and(|addressed| *addressed)
        {
            continue;
        }
        family.push(queue_row(
            athlete.id.as_str(),
            subject_of(
                &athlete.canonical_name,
                school_of(names, athlete.school.as_str()),
            ),
            "the athlete school carries no postal address claim; recruiting mail cannot be addressed".to_string(),
        ));
    }
    family
}

pub(in crate::workbook::meta::queues) fn transfer_ambiguous(
    cohort: &[CanonicalAthlete],
    source: &crate::export::ExportDataset,
    names: &HashMap<&str, &str>,
) -> crate::report::ReportResult<Family> {
    let mut family = Family::new(TRANSFER);
    if source.selected_profile_id().is_none() {
        return Ok(family);
    }
    let schools = crate::export::current_school::SelectedSchools::of(source)?;
    for athlete in cohort {
        if !schools.has_unresolved_current_school(athlete.id.as_str())? {
            continue;
        }
        family.push(queue_row(
            athlete.id.as_str(),
            subject_of(
                &athlete.canonical_name,
                school_of(names, athlete.school.as_str()),
            ),
            "the current school cannot be resolved from retained affiliations".to_string(),
        ));
    }
    Ok(family)
}

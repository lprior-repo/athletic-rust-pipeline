use crate::cli::export_data::csv::write_csv;
use census_domain::model::{CanonicalCoach, CanonicalSchool};
use std::collections::{BTreeSet, HashMap};

fn build_coach_row(c: &CanonicalCoach, by_school: &HashMap<&str, &CanonicalSchool>) -> Vec<String> {
    let school = by_school.get(c.school.as_str()).copied();
    let (source_url, observed_on) = census_report::export::coach_source(c);
    let evidence_src = c
        .evidence
        .iter()
        .map(|e| e.source.id.as_str())
        .collect::<BTreeSet<_>>()
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join(";");

    vec![
        c.id.as_str().to_string(),
        c.name.clone(),
        c.role.stable_key().to_string(),
        c.sport
            .map(|s| s.stable_key().to_string())
            .map_or(Default::default(), core::convert::identity),
        c.gender.stable_key().to_string(),
        c.school.as_str().to_string(),
        school
            .map(|school| school.name.clone())
            .map_or(Default::default(), core::convert::identity),
        school
            .and_then(|school| school.state)
            .map(|state| state.code().to_owned())
            .map_or(Default::default(), core::convert::identity),
        c.professional_email
            .clone()
            .map_or(Default::default(), core::convert::identity),
        c.personal_email
            .clone()
            .map_or(Default::default(), core::convert::identity),
        source_url
            .map_or(Default::default(), core::convert::identity)
            .to_owned(),
        evidence_src,
        observed_on
            .map_or(Default::default(), core::convert::identity)
            .to_owned(),
    ]
}

pub fn write_canonical_coaches(
    coaches: &[CanonicalCoach],
    schools: &[CanonicalSchool],
    data: &std::path::Path,
) -> anyhow::Result<()> {
    let by_school: HashMap<&str, &CanonicalSchool> =
        schools.iter().map(|s| (s.id.as_str(), s)).collect();

    let rows: Vec<Vec<String>> = coaches
        .iter()
        .map(|c| build_coach_row(c, &by_school))
        .collect();

    write_csv(
        &data.join("canonical-coaches.csv"),
        &[
            "coach_id",
            "name",
            "role",
            "sport",
            "gender",
            "school_id",
            "school_name",
            "school_state",
            "professional_email",
            "personal_email",
            "source_url",
            "evidence_sources",
            "observed_on",
        ],
        &rows,
    )?;

    Ok(())
}

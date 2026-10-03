use crate::cli::export_data::csv::write_csv;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, GradYear, SourceIdentity, SourceNamespace,
};
use std::collections::{BTreeSet, HashMap};

const ATHLETE_HEADERS: [&str; 18] = [
    "athlete_id",
    "name",
    "grad_year",
    "gender",
    "state",
    "school_id",
    "school_name",
    "school_city",
    "sports",
    "cohort_confidence",
    "athleticnet_athlete_id",
    "athleticnet_url",
    "milesplit_athlete_id",
    "milesplit_url",
    "profile_urls",
    "source_namespaces",
    "evidence_sources",
    "observed_grades",
];

const SEED_HEADERS: [&str; 12] = [
    "athleticnet_athlete_id",
    "athleticnet_url",
    "name",
    "grad_year",
    "gender",
    "state",
    "school_name",
    "city",
    "sports",
    "derived_from_sources",
    "milesplit_athlete_id",
    "observed_on",
];

fn athletic_net(athlete: &CanonicalAthlete) -> Option<&SourceIdentity> {
    athlete.identities().find(|identity| {
        matches!(&identity.namespace,
            SourceNamespace::AthleticNet { kind } | SourceNamespace::LegacyAthleticNet { kind }
            if kind == "athlete")
    })
}

fn milesplit(athlete: &CanonicalAthlete) -> Option<&SourceIdentity> {
    athlete
        .identities()
        .find(|identity| identity.namespace == SourceNamespace::MilesplitAthlete)
}

fn namespaces(athlete: &CanonicalAthlete) -> BTreeSet<&SourceNamespace> {
    athlete
        .identities()
        .map(|identity| &identity.namespace)
        .collect()
}

fn sports(athlete: &CanonicalAthlete) -> String {
    athlete
        .sports
        .iter()
        .map(|sport| sport.stable_key())
        .collect::<Vec<_>>()
        .join(";")
}

fn sources(athlete: &CanonicalAthlete) -> String {
    athlete
        .evidence
        .iter()
        .map(|evidence| evidence.source.id.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(";")
}

fn namespace_labels(athlete: &CanonicalAthlete) -> String {
    namespaces(athlete)
        .iter()
        .map(|namespace| namespace.to_string())
        .collect::<Vec<_>>()
        .join(";")
}

fn row(athlete: &CanonicalAthlete, school: Option<&CanonicalSchool>) -> Vec<String> {
    let an = athletic_net(athlete);
    let ms = milesplit(athlete);
    vec![
        athlete.id.as_str().to_owned(),
        athlete.canonical_name.clone(),
        athlete.grad_year.get().to_string(),
        athlete.gender.stable_key().to_owned(),
        school
            .and_then(|school| school.state)
            .map(|state| state.code().to_owned())
            .map_or(Default::default(), core::convert::identity),
        athlete.school.as_str().to_owned(),
        school
            .map(|school| school.name.clone())
            .map_or(Default::default(), core::convert::identity),
        school
            .and_then(|school| school.city.clone())
            .map_or(Default::default(), core::convert::identity),
        sports(athlete),
        athlete
            .derived_cohort_confidence()
            .map(|confidence| confidence.get().to_string())
            .map_or(Default::default(), core::convert::identity),
        an.map(|identity| identity.id.clone())
            .map_or(Default::default(), core::convert::identity),
        an.and_then(|identity| identity.url.clone())
            .map_or(Default::default(), core::convert::identity),
        ms.map(|identity| identity.id.clone())
            .map_or(Default::default(), core::convert::identity),
        ms.and_then(|identity| identity.url.clone())
            .map_or(Default::default(), core::convert::identity),
        athlete.public_profile_urls.join(";"),
        namespace_labels(athlete),
        sources(athlete),
        athlete
            .observed_grades
            .iter()
            .map(|grade| format!("g{}@{}", grade.grade.get(), grade.school_year.get()))
            .collect::<Vec<_>>()
            .join(";"),
    ]
}

fn seed_row(
    athlete: &CanonicalAthlete,
    school: Option<&CanonicalSchool>,
    source: &SourceIdentity,
) -> Vec<String> {
    vec![
        source.id.clone(),
        source
            .url
            .clone()
            .map_or(Default::default(), core::convert::identity),
        athlete.canonical_name.clone(),
        athlete.grad_year.get().to_string(),
        athlete.gender.stable_key().to_owned(),
        school
            .and_then(|school| school.state)
            .map(|state| state.code().to_owned())
            .map_or(Default::default(), core::convert::identity),
        school
            .map(|school| school.name.clone())
            .map_or(Default::default(), core::convert::identity),
        school
            .and_then(|school| school.city.clone())
            .map_or(Default::default(), core::convert::identity),
        sports(athlete),
        namespace_labels(athlete),
        milesplit(athlete)
            .map(|identity| identity.id.clone())
            .map_or(Default::default(), core::convert::identity),
        athlete
            .evidence
            .iter()
            .map(|evidence| evidence.observed_on.as_str())
            .max()
            .map_or(Default::default(), core::convert::identity)
            .to_owned(),
    ]
}

pub fn write_athletes(
    athletes: &[CanonicalAthlete],
    schools: &[CanonicalSchool],
    data: &std::path::Path,
) -> anyhow::Result<(usize, usize)> {
    let schools: HashMap<&str, &CanonicalSchool> = schools
        .iter()
        .map(|school| (school.id.as_str(), school))
        .collect();
    let mut cohort = Vec::new();
    let mut seeds = Vec::new();
    let mut multi_source = 0usize;
    for athlete in athletes {
        let school = schools.get(athlete.school.as_str()).copied();
        if namespaces(athlete).len() > 1 {
            multi_source = multi_source
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("multi-source count exceeds usize"))?;
        }
        if let Some(source) = athletic_net(athlete) {
            seeds.push(seed_row(athlete, school, source));
        }
        if athlete.grad_year == GradYear::CO2027 {
            cohort.push(row(athlete, school));
        }
    }
    write_csv(
        &data.join("canonical-athletes-co2027.csv"),
        &ATHLETE_HEADERS,
        &cohort,
    )?;
    write_csv(
        &data.join("athleticnet-athlete-seeds.csv"),
        &SEED_HEADERS,
        &seeds,
    )?;
    Ok((cohort.len(), multi_source))
}

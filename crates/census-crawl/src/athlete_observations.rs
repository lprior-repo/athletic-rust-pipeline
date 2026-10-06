use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, SourceAthleteObservation, SourceObservation,
};
use std::collections::HashMap;

pub fn athlete_observations_of<'a>(
    athletes: &[CanonicalAthlete],
    schools: impl IntoIterator<Item = &'a CanonicalSchool>,
    observed_on: &str,
) -> Vec<SourceObservation> {
    let mut names: HashMap<&str, &str> = HashMap::new();
    for school in schools {
        names
            .entry(school.id.as_str())
            .or_insert(school.name.as_str());
    }
    athletes
        .iter()
        .filter_map(|athlete| {
            let school = names
                .get(athlete.school.as_str())
                .map(|name| (*name).to_string());
            let source = athlete.source.as_ref()?;
            SourceAthleteObservation::of_athlete(&source.namespace, athlete, school, observed_on)
        })
        .map(SourceObservation::Athlete)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::{Gender, GradYear, SourceIdentity, SourceNamespace};
    use census_domain::UsJurisdiction;
    use std::collections::BTreeSet;

    fn school() -> CanonicalSchool {
        CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Example School",
            "example school",
            None,
        )
        .0
    }

    fn athlete(school: &CanonicalSchool, source: SourceIdentity) -> CanonicalAthlete {
        CanonicalAthlete::new(
            &school.id,
            "Alex Rivera",
            GradYear::CO2027,
            Gender::Boys,
            source,
        )
    }

    #[test]
    fn primary_observations_preserve_provider_homonyms_despite_advisory_links() {
        let school = school();
        let first_source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "11111");
        let mut first = athlete(&school, first_source.clone());
        first.add_identity(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "99999"));
        let mut second = athlete(
            &school,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "22222"),
        );
        second.add_identity(first_source);
        let rows = athlete_observations_of(&[first, second], [&school], "2026-09-26");
        let identities: BTreeSet<_> = rows
            .iter()
            .map(|row| match row {
                SourceObservation::Athlete(row) => {
                    assert_eq!(row.namespace, SourceNamespace::MilesplitAthlete);
                    row.source_athlete_id.as_str()
                }
                other => panic!("unexpected observation: {other:?}"),
            })
            .collect();
        assert_eq!(identities, BTreeSet::from(["11111", "22222"]));
        assert_eq!(rows.len(), identities.len());
    }

    #[test]
    fn a_result_row_remains_row_owned_despite_a_linked_person_identifier() {
        let school = school();
        let namespace = SourceNamespace::Other("timer_result_row".to_owned());
        let source = SourceIdentity::new(namespace.clone(), "meet:5:event:2:row:9")
            .with_url("https://example.test/meet/5/event/2");
        let mut row = athlete(&school, source);
        row.add_identity(SourceIdentity::new(
            SourceNamespace::MilesplitAthlete,
            "11111",
        ));
        let observations = athlete_observations_of(&[row], [&school], "2026-09-26");
        let [SourceObservation::Athlete(observation)] = observations.as_slice() else {
            panic!("expected exactly one row-owned observation");
        };
        assert_eq!(observation.namespace, namespace);
        assert_eq!(observation.source_athlete_id, "meet:5:event:2:row:9");
        assert_eq!(
            observation.source_row_key,
            "https://example.test/meet/5/event/2"
        );
    }
}

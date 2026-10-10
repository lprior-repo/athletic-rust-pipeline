use census_domain::model::{CanonicalCoach, CoachId};
use census_store::Entity;
use std::collections::HashMap;

pub(in crate::workbook) fn claims(observations: &[CanonicalCoach]) -> Vec<CanonicalCoach> {
    let mut first_occurrence: HashMap<CoachId, usize> = HashMap::new();
    let mut output = Vec::new();
    for observation in observations {
        match first_occurrence.entry(observation.id.clone()) {
            std::collections::hash_map::Entry::Vacant(vacant) => {
                vacant.insert(output.len());
                output.push(observation.clone());
            }
            std::collections::hash_map::Entry::Occupied(occupied) => {
                let index = *occupied.get();
                output[index].merge(observation.clone());
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::{CoachRole, Gender, SchoolId, Sport};

    fn coach(name: &str, id: CoachId) -> CanonicalCoach {
        let mut coach = CanonicalCoach::new(
            &SchoolId::mint("sch", &["coach claim fixture"]),
            name,
            Some(Sport::OutdoorTrack),
            Gender::Mixed,
            CoachRole::HeadCoach,
        );
        coach.id = id;
        coach
    }

    fn with_email(mut coach: CanonicalCoach, email: &str) -> CanonicalCoach {
        coach.set_published_email(email);
        coach
    }

    fn different_email_coach(name: &str, id: CoachId, email: &str) -> CanonicalCoach {
        with_email(coach(name, id), email)
    }

    #[test]
    fn duplicate_observations_produce_one_claim() {
        let id = mint_id("duplicate");
        let first = coach("Alice", id.clone());
        let second = coach("Alice", id.clone());
        let result = claims(&[first, second]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, id);
    }

    #[test]
    fn distinct_coaches_produce_distinct_claims() {
        let id1 = mint_id("coach-1");
        let id2 = mint_id("coach-2");
        let first = with_email(coach("Alice", id1), "alice@example.test");
        let second = with_email(coach("Bob", id2), "bob@example.test");
        let result = claims(&[first, second]);
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].professional_email,
            Some("alice@example.test".to_string())
        );
        assert_eq!(
            result[1].professional_email,
            Some("bob@example.test".to_string())
        );
    }

    #[test]
    fn merged_observations_keep_the_first_published_email() {
        let id = mint_id("routing");
        let first = with_email(coach("Charlie", id.clone()), "charlie@example.test");
        let second = different_email_coach("Charlie", id, "charlie-later@example.test");
        let result = claims(&[first, second]);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].professional_email,
            Some("charlie@example.test".to_string())
        );
    }

    #[test]
    fn same_school_different_sport_not_merged() {
        let id1 = mint_id("sport-1");
        let id2 = mint_id("sport-2");
        let first = coach("Dana", id1);
        let mut second = coach("Dana", id2);
        second.sport = Some(Sport::CrossCountry);
        let result = claims(&[first, second]);
        assert_eq!(result.len(), 2);
    }

    fn mint_id(key: &str) -> CoachId {
        let school = SchoolId::mint("sch", &["coach mint fixture"]);
        let coach = CanonicalCoach::new(
            &school,
            key,
            Some(Sport::OutdoorTrack),
            Gender::Mixed,
            CoachRole::HeadCoach,
        );
        coach.id
    }
}

use super::{owner_of, resolve_school, Candidate, Parents, Scope};
use crate::bests::{SelectionAthlete, SelectionMeet, SelectionResult, SelectionSource};
use census_domain::model::EventSpecification;
use census_domain::{JurisdictionBucket, MeetState};

impl Candidate<'_> {
    pub(super) fn result(&self) -> SelectionResult {
        let performance = self.performance;
        SelectionResult {
            value: self.value,
            normalized: self.measure.normalized_mark(&performance.mark),
            mark: performance.mark.clone(),
            place: performance.place,
            wind_mps: performance.wind_mps,
            timing: performance.timing,
        }
    }

    pub(super) fn meet(&self) -> SelectionMeet {
        SelectionMeet {
            date: self.performance.date.clone(),
            name: self.meet.map_or_else(String::new, |meet| meet.name.clone()),
            meet_id: self.performance.meet.clone(),
            meet_state: MeetState::from(self.meet.and_then(|meet| meet.state)),
        }
    }

    pub(super) fn source(
        &self,
        scope: Scope,
        specification: EventSpecification,
    ) -> SelectionSource {
        let performance = self.performance;
        SelectionSource {
            specification,
            result_url: scope
                .primary_evidence(performance)
                .and_then(|evidence| evidence.source.url.clone())
                .map_or_else(String::new, core::convert::identity),
            performance_id: performance.id.clone(),
            source_athlete: owner_of(performance, self.athlete)
                .map_or_else(String::new, |identity| identity.id.clone()),
            source_key: performance.source_key.clone(),
        }
    }

    pub(super) fn athlete(&self, parents: &Parents<'_>) -> SelectionAthlete {
        let athlete = self.athlete;
        SelectionAthlete {
            name: athlete.canonical_name.clone(),
            gender: athlete.gender,
            grad_year: athlete.grad_year.get(),
            profile_url: athlete.public_profile_urls.first().cloned(),
            school: resolve_school(&self.performance.team, parents),
            athlete_school: athlete.school.as_str().to_string(),
            athlete_state: JurisdictionBucket::from(
                parents
                    .school(athlete.school.as_str())
                    .and_then(|school| school.state),
            ),
        }
    }
}

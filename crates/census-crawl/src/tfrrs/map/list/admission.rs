use super::{Absorb, ListContext, RowFacts};
use crate::tfrrs::map::entity::athlete_source;
use census_domain::model::{
    Gender, GradYear, Grade, ObservedGrade, SchoolYear, SourceAthleteObservation, SourceIdentity,
};

pub(super) fn athlete(
    absorb: &mut Absorb<'_>,
    context: &ListContext<'_>,
    facts: &RowFacts<'_>,
    observed: (Grade, SchoolYear, Gender),
    source_key: &str,
) -> Option<(GradYear, ObservedGrade, SourceIdentity)> {
    let (grade, school_year, gender) = observed;
    let observation = ObservedGrade {
        grade,
        school_year,
        source: context.page.source.clone(),
    };
    let source = athlete_source(facts.athlete.id, source_key);
    absorb
        .accumulator
        .unsupported
        .admit(observation, source, |source| {
            SourceAthleteObservation::new(
                source.namespace,
                source.id,
                source_key,
                facts.name,
                context.page.observed_on,
            )
            .with_school(Some(facts.team.name.clone()))
            .with_gender(gender)
        })
}

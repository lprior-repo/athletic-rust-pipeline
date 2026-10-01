use census_domain::model::{
    GradYear, ObservedGrade, ReviewCase, SourceAthleteObservation, SourceIdentity,
    SourceObservation, UNSUPPORTED_GRADUATION_FAMILY,
};

pub(crate) struct UnsupportedCohort<'a>(&'a ObservedGrade);

impl UnsupportedCohort<'_> {
    pub fn review_case(
        &self,
        source_row: &str,
        name: &str,
        school: &str,
        observed_on: &str,
    ) -> ReviewCase {
        let observation = self.0;
        let source_url = observation.source.url.as_deref().map_or("", |url| url);
        let detail = format!(
            "Published grade {} in school year {} does not imply a supported graduation year ({}..={}). School: {school}. Source: {}. URL: {source_url}. Row: {source_row}. Observed on: {observed_on}. No canonical graduation year or identity acceptance was assigned.",
            observation.grade,
            observation.school_year.get(),
            GradYear::MIN_YEAR,
            GradYear::MAX_YEAR,
            observation.source.id,
        );
        ReviewCase::pending(
            UNSUPPORTED_GRADUATION_FAMILY,
            format!("{}:{source_row}", observation.source.id),
            name,
            detail,
        )
    }
}

#[derive(Debug, Default)]
pub(crate) struct UnsupportedCohortRows {
    cases: Vec<ReviewCase>,
    observations: Vec<SourceObservation>,
}

impl UnsupportedCohortRows {
    pub fn admit(
        &mut self,
        observation: ObservedGrade,
        source: SourceIdentity,
        row: impl FnOnce(SourceIdentity) -> SourceAthleteObservation,
    ) -> Option<(GradYear, ObservedGrade, SourceIdentity)> {
        match observation.grad_year() {
            Some(year) => Some((year, observation, source)),
            None => {
                self.retain(observation, row(source));
                None
            }
        }
    }

    pub fn into_parts(self) -> (Vec<ReviewCase>, Vec<SourceObservation>) {
        (self.cases, self.observations)
    }

    pub fn retain(&mut self, observation: ObservedGrade, row: SourceAthleteObservation) {
        if observation.grad_year().is_none() {
            self.cases.push(UnsupportedCohort(&observation).review_case(
                &row.source_row_key,
                &row.observed_name,
                row.observed_school.as_deref().map_or("", |school| school),
                &row.observed_on,
            ));
        }
        self.observations.push(SourceObservation::Athlete(
            row.with_grade(Some(observation)),
        ));
    }

    pub fn len(&self) -> usize {
        self.cases.len()
    }

    pub fn append_to(&self, batch: &mut crate::recording::RowBatch<'_>) -> crate::CrawlResult<()> {
        batch.append_many(census_store::Table::ReviewCases, &self.cases)?;
        batch.append_many(census_store::Table::SourceObservations, &self.observations)?;
        Ok(())
    }
}

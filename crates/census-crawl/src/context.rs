use crate::athlete_observations::athlete_observations_of;
use crate::net::{FetchOptions, Fetcher};
use crate::recording::{Recording, RowBatch, RowSink};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, SourceNamespace, SourceObservation, SourceSchoolObservation,
};
use census_store::Store;

pub struct AdapterContext<'a> {
    pub fetcher: &'a Fetcher,
    pub store: &'a Store,
    pub refresh: bool,
    pub school_year: census_domain::model::SchoolYear,
    pub observed_on: String,
    pub recording: Option<&'a Recording>,
}

impl<'a> AdapterContext<'a> {
    pub fn write_batch(&self) -> RowBatch<'_> {
        self.sink().write_batch()
    }

    fn sink(&self) -> RowSink<'_> {
        match self.recording {
            Some(recording) => RowSink::Record(recording),
            None => RowSink::Store(self.store),
        }
    }

    pub fn fetch_options(&self) -> FetchOptions {
        FetchOptions {
            refresh: self.refresh,
            allow_not_found: false,
            headers: Vec::new(),
        }
    }

    pub fn school_observations(
        &self,
        namespace: &SourceNamespace,
        schools: &[CanonicalSchool],
    ) -> Vec<SourceObservation> {
        school_observations_of(namespace, schools, &self.observed_on)
    }

    pub fn school_observation(
        &self,
        namespace: &SourceNamespace,
        school: &CanonicalSchool,
    ) -> Option<SourceObservation> {
        SourceSchoolObservation::of_school(namespace, school, &self.observed_on)
            .map(SourceObservation::School)
    }

    pub fn athlete_observations<'s>(
        &self,
        athletes: &[CanonicalAthlete],
        schools: impl IntoIterator<Item = &'s CanonicalSchool>,
    ) -> Vec<SourceObservation> {
        athlete_observations_of(athletes, schools, &self.observed_on)
    }
}

pub fn school_observations_of(
    namespace: &SourceNamespace,
    schools: &[CanonicalSchool],
    observed_on: &str,
) -> Vec<SourceObservation> {
    schools
        .iter()
        .filter_map(|school| SourceSchoolObservation::of_school(namespace, school, observed_on))
        .map(SourceObservation::School)
        .collect()
}


#[cfg(test)]
mod observation_tests;

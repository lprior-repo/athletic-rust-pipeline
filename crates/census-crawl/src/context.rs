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
    pub performance_as_of: chrono::NaiveDate,
    pub recording: Option<&'a Recording>,
}

pub fn assess_performance_date(
    as_of: chrono::NaiveDate,
    published: &str,
) -> PerformanceDateAssessment {
    match published_performance_date(published) {
        Some(date) if date <= as_of => PerformanceDateAssessment::Admitted,
        Some(_) => PerformanceDateAssessment::Future,
        None => PerformanceDateAssessment::Unknown,
    }
}

pub(crate) fn published_performance_date(published: &str) -> Option<chrono::NaiveDate> {
    if published.len() > 64 {
        return None;
    }
    let prefix = published.get(..10)?;
    if !prefix.bytes().enumerate().all(|(index, byte)| {
        if matches!(index, 4 | 7) {
            byte == b'-'
        } else {
            byte.is_ascii_digit()
        }
    }) {
        return None;
    }
    let date = chrono::NaiveDate::parse_from_str(prefix, "%Y-%m-%d").ok()?;
    if published.len() == 10 {
        return Some(date);
    }
    let instant =
        chrono::DateTime::parse_from_rfc3339(published).map(|instant| instant.date_naive());
    let local = || {
        chrono::NaiveDateTime::parse_from_str(published, "%Y-%m-%dT%H:%M:%S%.f")
            .map(|instant| instant.date())
    };
    instant
        .or_else(|_| local())
        .ok()
        .filter(|parsed| *parsed == date)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceDateAssessment {
    Admitted,
    Future,
    Unknown,
}

impl<'a> AdapterContext<'a> {
    pub fn assess_performance_date(&self, date: &str) -> PerformanceDateAssessment {
        assess_performance_date(self.performance_as_of, date)
    }

    pub fn effect_is_committed(&self, operation: &str, digest: &str) -> crate::CrawlResult<bool> {
        self.sink().effect_is_committed(operation, digest)
    }

    pub(crate) fn append_row_once<T: serde::Serialize>(
        &self,
        phase: &str,
        table: census_store::Table,
        row: &T,
    ) -> crate::CrawlResult<bool> {
        crate::recording::row::append_once(self, phase, table, row)
    }

    pub fn write_batch(&self) -> RowBatch<'_> {
        self.sink().write_batch()
    }

    fn sink(&self) -> RowSink<'_> {
        match self.recording {
            Some(recording) => RowSink::Record {
                store: self.store,
                recording,
            },
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

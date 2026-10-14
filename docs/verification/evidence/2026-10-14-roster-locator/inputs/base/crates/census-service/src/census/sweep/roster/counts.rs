use census_crawl::milesplit::RosterAthlete;
use census_crawl::{CrawlError, CrawlResult};
use census_domain::model::{CanonicalTeam, Gender, GradYear, Sport};
use std::collections::HashSet;

const MAX_PROGRAMS: usize = 12;

#[derive(Default)]
pub(super) struct Counts {
    pub(super) athletes: usize,
    pub(super) co2027: usize,
    pub(super) boys: usize,
    pub(super) girls: usize,
    programs: HashSet<(Sport, Gender)>,
}

impl Counts {
    pub(super) fn new() -> CrawlResult<Self> {
        let mut counts = Self::default();
        counts
            .programs
            .try_reserve(MAX_PROGRAMS)
            .map_err(|_| resource(MAX_PROGRAMS))?;
        Ok(counts)
    }

    pub(super) fn teams(&self) -> usize {
        self.programs.len()
    }

    pub(super) fn include(
        &mut self,
        rows: &[RosterAthlete],
        teams: &[CanonicalTeam],
    ) -> CrawlResult<()> {
        self.athletes = add(self.athletes, rows.len())?;
        rows.iter()
            .filter(|row| row.grad_year == GradYear::CO2027)
            .try_for_each(|row| {
                self.co2027 = add(self.co2027, 1)?;
                match row.gender {
                    Gender::Boys => self.boys = add(self.boys, 1)?,
                    Gender::Girls => self.girls = add(self.girls, 1)?,
                    Gender::Mixed | Gender::Unknown => {}
                }
                Ok::<_, CrawlError>(())
            })?;
        teams.iter().try_for_each(|team| {
            let key = (team.sport, team.gender);
            if self.programs.len() >= MAX_PROGRAMS && !self.programs.contains(&key) {
                return Err(resource(self.programs.len()));
            }
            self.programs.insert(key);
            Ok(())
        })
    }
}

fn add(left: usize, right: usize) -> CrawlResult<usize> {
    left.checked_add(right)
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "roster prefix count overflowed".to_string(),
        })
}
fn resource(requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "roster programs",
        requested,
        limit: MAX_PROGRAMS,
    }
}
